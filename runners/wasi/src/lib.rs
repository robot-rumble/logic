use pin_project_lite::pin_project;
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::{io, task};
use wasmer_runtime::{error::CallError, Instance};
use wasmer_wasi::state::WasiStateBuilder;

mod stdio;
pub use stdio::{Stderr, Stdin, Stdout};

pub fn add_stdio(state: &mut WasiStateBuilder) -> &mut WasiStateBuilder {
    state
        .stdin(Box::new(stdio::Stdin))
        .stdout(Box::new(stdio::Stdout))
        .stderr(Box::new(stdio::Stdout))
}

type Buf = Cursor<Vec<u8>>;
type StdinInner = io::Result<Buf>;
tokio::task_local! {
    static STDIN: Arc<Mutex<io::StreamReader<mpsc::Receiver<StdinInner>, Buf>>>;
    static STDOUT: Arc<Mutex<mpsc::Sender<StdinInner>>>;
    static STDERR: Arc<Mutex<mpsc::Sender<StdinInner>>>;
}

pin_project! {
    pub struct WasiStdinWriter {
        tx: Option<mpsc::Sender<StdinInner>>,
    }
}

impl AsyncWrite for WasiStdinWriter {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.project();
        let tx = match this.tx {
            Some(tx) => tx,
            None => {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    "called write after shutdown",
                )))
            }
        };
        tx.poll_ready(cx).map(|res| {
            let kind = io::ErrorKind::BrokenPipe; // ?
            res.map_err(|e| io::Error::new(kind, e))
                .and_then(|()| {
                    tx.try_send(Ok(Cursor::new(buf.to_owned())))
                        .map_err(|e| io::Error::new(kind, e))
                })
                .map(|()| buf.len())
        })
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        self.project().tx.take().map(drop);
        Poll::Ready(Ok(()))
    }
}

pin_project! {
    pub struct WasiStdoutReader {
        #[pin]
        inner: io::StreamReader<mpsc::Receiver<StdinInner>, Buf>,
    }
}

impl AsyncRead for WasiStdoutReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.project().inner.poll_read(cx, buf)
    }
}

pin_project! {
    pub struct WasiProcess {
        in_tx: Option<mpsc::Sender<StdinInner>>,
        out_rx: Option<mpsc::Receiver<StdinInner>>,
        err_rx: Option<mpsc::Receiver<StdinInner>>,
        handle: futures::future::BoxFuture<'static, Result<(), CallError>>,
    }
}

impl WasiProcess {
    pub fn spawn(instance: Instance) -> Self {
        let (in_tx, in_rx) = mpsc::channel(5);
        let (out_tx, out_rx) = mpsc::channel(5);
        let (err_tx, err_rx) = mpsc::channel(5);
        let handle = STDIN.scope(
            Arc::new(Mutex::new(io::stream_reader(in_rx))),
            STDOUT.scope(
                Arc::new(Mutex::new(out_tx)),
                STDERR.scope(Arc::new(Mutex::new(err_tx)), async move {
                    task::block_in_place(|| instance.call("_start", &[]).map(drop))
                }),
            ),
        );

        Self {
            in_tx: Some(in_tx),
            out_rx: Some(out_rx),
            err_rx: Some(err_rx),
            handle: Box::pin(handle),
        }
    }

    pub fn take_stdin(&mut self) -> Option<WasiStdinWriter> {
        self.in_tx.take().map(|tx| WasiStdinWriter { tx: Some(tx) })
    }
    pub fn take_stdout(&mut self) -> Option<WasiStdoutReader> {
        self.out_rx.take().map(|rx| WasiStdoutReader {
            inner: io::stream_reader(rx),
        })
    }
    pub fn take_stderr(&mut self) -> Option<WasiStdoutReader> {
        self.err_rx.take().map(|rx| WasiStdoutReader {
            inner: io::stream_reader(rx),
        })
    }
}

impl Future for WasiProcess {
    type Output = Result<(), CallError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.project().handle.as_mut().poll(cx)
    }
}
