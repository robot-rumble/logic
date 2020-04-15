use std::io::{self, prelude::*};
use std::path;
use std::process::{Command, Stdio};

use logic::{ProgramError, Team};

use maplit::hashmap;

fn make_command_f(mut command: Command) -> impl FnMut(logic::ProgramInput) -> logic::ProgramOutput {
    let mut proc = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = io::BufWriter::new(proc.stdin.take().unwrap());
    let mut stdout = io::BufReader::new(proc.stdout.take().unwrap());

    move |inp| run_stdio(&mut stdin, &mut stdout, inp)
}

fn main() {
    let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("robot.py");

    let make_cmd = || {
        let mut cmd = Command::new("python");
        cmd.arg(&path);
        cmd
    };

    let runmap = hashmap! {
        Team::Red => make_command_f(make_cmd()),
        Team::Blue => make_command_f(make_cmd()),
    };

    let output = logic::run(
        runmap,
        |turn_state| {
            println!("{}", turn_state.state.state);
        },
        10,
    );
    println!("Output: {:?}", output);
}

fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.starts_with(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn run_stdio(
    mut stdin: impl Write,
    stdout: impl BufRead,
    input: logic::ProgramInput,
) -> logic::ProgramOutput {
    let mut logs = Vec::new();
    let run = || -> logic::ProgramResult {
        let mut lines = stdout.lines();
        let output: logic::ProgramOutput = loop {
            serde_json::to_writer(&mut stdin, &input)?;
            stdin.write(b"\n")?;
            stdin.flush()?;
            let line = lines.next().ok_or(ProgramError::NoData)??;
            if let Some(output) = strip_prefix(&line, "__rr_output:") {
                break serde_json::from_str(output)?;
            } else {
                logs.push(line)
            }
        };
        logs.extend(output.logs);
        output.robot_outputs
    };

    logic::ProgramOutput {
        robot_outputs: run(),
        logs,
    }
}
