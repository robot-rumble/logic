use std::process::Stdio;
use tokio::io;
use tokio::prelude::*;
use tokio::process::{ChildStdin, ChildStdout, Command};

use logic::{ProgramError, RunnerError};

pub struct TokioRunner<W: AsyncWrite, R: AsyncBufRead> {
    stdin: W,
    stdout: R,
}

impl TokioRunner<io::BufWriter<ChildStdin>, io::BufReader<ChildStdout>> {
    pub async fn new_cmd(mut command: Command) -> Result<Self, ProgramError> {
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
impl<W: AsyncWrite + Unpin, R: AsyncBufRead + Unpin> TokioRunner<W, R> {
    pub async fn new(stdin: W, mut stdout: R) -> Result<Self, ProgramError> {
        let line: String = (&mut stdout)
            .lines()
            .next_line()
            .await?
            .ok_or(ProgramError::NoData)?;
        let init_result = strip_prefix(&line, "__rr_init:").ok_or(ProgramError::NoInitError)?;
        serde_json::from_str::<Result<(), ProgramError>>(init_result)??;

        Ok(Self { stdin, stdout })
    }
}

#[async_trait::async_trait(?Send)]
impl<W: AsyncWrite + Unpin, R: AsyncBufRead + Unpin> logic::RobotRunner for TokioRunner<W, R> {
    async fn run(&mut self, input: logic::ProgramInput) -> logic::RunnerResult {
        let mut input = serde_json::to_vec(&input)?;
        input.push(b'\n');
        self.stdin.write(&input).await?;
        self.stdin.flush().await?;

        let mut logs = Vec::new();
        let mut lines = (&mut self.stdout).lines();
        let mut output: logic::ProgramOutput = loop {
            macro_rules! try_with_logs {
                ($result:expr) => {
                    match $result {
                        Ok(ret) => ret,
                        Err(e) => return Err(RunnerError::new(e, logs)),
                    }
                };
            }
            let maybe_line = try_with_logs!(lines.next_line().await);
            let line = try_with_logs!(maybe_line.ok_or(ProgramError::NoData));
            if let Some(output) = strip_prefix(&line, "__rr_output:") {
                break try_with_logs!(serde_json::from_str(output));
            } else {
                logs.push(line)
            }
        };
        output.logs.extend(logs);
        Ok(output)
    }
}

fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.starts_with(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}
