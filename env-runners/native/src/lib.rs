use std::process::Stdio;
use tokio::io::{self, AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::time;

use logic::{ProgramError, ProgramResult};

pub struct TokioRunner<W: AsyncWrite, R: AsyncBufRead> {
    stdin: W,
    stdout: R,
}

pub type CommandRunner = TokioRunner<io::BufWriter<ChildStdin>, io::BufReader<ChildStdout>>;

impl CommandRunner {
    pub async fn new_cmd(mut command: Command) -> ProgramResult<Self> {
        let mut proc = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        let stdin = io::BufWriter::new(proc.stdin.take().unwrap());
        let stdout = io::BufReader::new(proc.stdout.take().unwrap());
        Self::new(stdin, stdout).await
    }
}
impl<W: AsyncWrite + Unpin + Send, R: AsyncBufRead + Unpin + Send> TokioRunner<W, R> {
    pub async fn new(stdin: W, mut stdout: R) -> ProgramResult<Self> {
        let line: String = (&mut stdout)
            .lines()
            .next_line()
            .await?
            .ok_or(ProgramError::NoData)?;
        let init_result = line
            .strip_prefix("__rr_init:")
            .ok_or(ProgramError::NoInitError)?;
        serde_json::from_str::<ProgramResult<()>>(init_result)??;

        Ok(Self { stdin, stdout })
    }
}

#[async_trait::async_trait]
impl<W: AsyncWrite + Unpin + Send, R: AsyncBufRead + Unpin + Send> logic::RobotRunner
    for TokioRunner<W, R>
{
    async fn run(&mut self, input: logic::ProgramInput<'_>) -> ProgramResult {
        let mut input = serde_json::to_vec(&input)?;
        input.push(b'\n');
        self.stdin.write_all(&input).await?;
        self.stdin.flush().await?;

        let mut logs = Vec::new();
        let mut lines = (&mut self.stdout).lines();
        let mut res = loop {
            let maybe_line = lines.next_line().await?;
            let line = maybe_line.ok_or(ProgramError::NoData)?;
            if let Some(output) = line.strip_prefix("__rr_output:") {
                break serde_json::from_str::<ProgramResult>(output)?;
            } else {
                logs.push(line)
            }
        };
        if let Ok(output) = &mut res {
            output.logs.extend(logs);
        }
        res
    }
}

pub struct TimeoutRunner<R: logic::RobotRunner> {
    inner: R,
    timeout: Option<time::Duration>,
}

impl<R: logic::RobotRunner> TimeoutRunner<R> {
    pub fn new(inner: R, timeout: Option<time::Duration>) -> Self {
        Self { inner, timeout }
    }
}

#[async_trait::async_trait]
impl<R: logic::RobotRunner + Send> logic::RobotRunner for TimeoutRunner<R> {
    async fn run(&mut self, input: logic::ProgramInput<'_>) -> ProgramResult {
        let fut = self.inner.run(input);
        if let Some(dur) = self.timeout {
            time::timeout(dur, fut)
                .await
                .unwrap_or(Err(ProgramError::Timeout(dur)))
        } else {
            fut.await
        }
    }
}
