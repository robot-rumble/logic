use std::io::{self, prelude::*};
use std::path;
use std::process::{Command, Stdio};

use thiserror::Error;

fn main() {
    let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("robot.py");

    let mut child = Command::new("python")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut stdin = io::BufWriter::new(child.stdin.take().unwrap());
    let mut stdout = io::BufReader::new(child.stdout.take().unwrap());

    match logic::run(
        |robot_input| run(&mut stdin, &mut stdout, robot_input),
        |turn_state| {
            println!("{}", turn_state.state);
        },
        10,
    ) {
        Ok(output) => println!("Completed successfully, {:?} won", output.winner),
        Err(e) => eprintln!("Error: {}", e),
    };
}

#[derive(Error, Debug)]
enum Error {
    #[error("serde error")]
    Serde(#[from] serde_json::Error),
    #[error("io error")]
    Io(#[from] io::Error),
    #[error("process closed before printing an action")]
    NoAction,
}

fn run(
    mut stdin: impl Write,
    mut stdout: impl Read,
    input: logic::ProgramInput,
) -> Result<logic::ProgramOutput, Error> {
    let actions = input.state.teams[&input.team]
        .iter()
        .map(|id| -> Result<_, Error> {
            serde_json::to_writer(&mut stdin, &input)?;
            stdin.write(b"\n")?;
            stdin.flush()?;
            let action = serde_json::Deserializer::from_reader(&mut stdout)
                .into_iter()
                .next()
                .ok_or(Error::NoAction)??;
            Ok((*id, action))
        })
        .collect::<Result<_, _>>()?;

    Ok(logic::ProgramOutput { actions })
}
