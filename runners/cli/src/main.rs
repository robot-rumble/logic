use std::process::Stdio;
use tokio::io;
use tokio::prelude::*;
use tokio::process::{ChildStdin, ChildStdout, Command};

use itertools::Itertools;
use logic::{ProgramError, RunnerError};

struct CliRunner {
    stdin: io::BufWriter<ChildStdin>,
    stdout: io::BufReader<ChildStdout>,
}

impl CliRunner {
    async fn new(mut command: Command) -> Result<Self, ProgramError> {
        let mut proc = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        let stdin = io::BufWriter::new(proc.stdin.take().unwrap());
        let mut stdout = io::BufReader::new(proc.stdout.take().unwrap());

        let line = (&mut stdout)
            .lines()
            .next_line()
            .await?
            .ok_or(ProgramError::NoData)?;
        let init_result = strip_prefix(&line, "__rr_init:").ok_or(ProgramError::NoInitError)?;
        let init_result: Result<(), ProgramError> = serde_json::from_str(init_result)?;
        init_result?;

        Ok(Self { stdin, stdout })
    }
}

#[async_trait::async_trait]
impl logic::RobotRunner for CliRunner {
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

#[tokio::main]
async fn main() {
    let make_cmd = || {
        let mut args = std::env::args_os();
        let command = args.nth(1).expect("You must pass a command to run");
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        cmd
    };

    let output = logic::run(
        CliRunner::new(make_cmd()).await,
        CliRunner::new(make_cmd()).await,
        |turn_state| {
            println!(
                "State after turn {turn}:\n{logs}\nOutputs: {outputs:?}\nMap:\n{map}",
                turn = turn_state.state.turn,
                logs = turn_state
                    .logs
                    .iter()
                    .format_with("\n", |(team, logs), f| f(&format_args!(
                        "Logs for {:?}:\n{}",
                        team,
                        logs.iter().map(|s| s.trim()).format("\n"),
                    ))),
                outputs = turn_state.robot_outputs,
                map = turn_state.state.state,
            );
        },
        10,
    )
    .await;
    println!("Output: {:?}", output);
}

fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.starts_with(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}
