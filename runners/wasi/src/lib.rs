use pin_project_lite::pin_project;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
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

tokio::task_local! {
    static STDIN: Rc<RefCell<mpsc::Receiver<Vec<u8>>>>;
    static STDOUT: Rc<RefCell<mpsc::Sender<Vec<u8>>>>;
    static STDERR: Rc<RefCell<mpsc::Sender<Vec<u8>>>>;
}

pin_project! {
    struct MpscWriter {
        tx: Option<mpsc::Sender<Vec<u8>>>,
    }
}

impl AsyncWrite for MpscWriter {
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
                    tx.try_send(buf.to_owned())
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
    pub struct WasiProcess {
        in_tx: Option<mpsc::Sender<Vec<u8>>>,
        out_rx: Option<mpsc::Receiver<Vec<u8>>>,
        err_rx: Option<mpsc::Receiver<Vec<u8>>>,
        handle: futures::future::LocalBoxFuture<'static, Result<(), CallError>>,
    }
}

impl WasiProcess {
    pub fn spawn(instance: Instance) -> Self {
        let (in_tx, in_rx) = mpsc::channel(5);
        let (out_tx, out_rx) = mpsc::channel(5);
        let (err_tx, err_rx) = mpsc::channel(5);
        let handle = STDIN.scope(
            Rc::new(RefCell::new(in_rx)),
            STDOUT.scope(
                Rc::new(RefCell::new(out_tx)),
                STDERR.scope(Rc::new(RefCell::new(err_tx)), async move {
                    task::block_in_place(move || instance.call("_start", &[]).map(drop))
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

    pub fn take_stdin(&mut self) -> Option<impl AsyncWrite> {
        self.in_tx.take().map(|tx| MpscWriter { tx: Some(tx) })
    }
    pub fn take_stdout(&mut self) -> Option<impl AsyncRead> {
        self.out_rx.take().map(stdio::mpsc_reader)
    }
    pub fn take_stderr(&mut self) -> Option<impl AsyncRead> {
        self.err_rx.take().map(stdio::mpsc_reader)
    }
}

impl Future for WasiProcess {
    type Output = Result<(), CallError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.project().handle.as_mut().poll(cx)
    }
}
