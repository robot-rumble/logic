use std::io::Write;
use std::path;
use std::process::{Command, Stdio};

fn main() {
    let mut path = path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    path.push("robot.py");

    let mut child = Command::new("python")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    match logic::run(
        |team, robot_input| run(&mut child, team, robot_input),
        |turn_state| {
            println!("{:?}", turn_state);
        },
        |final_state| {
            println!("{:?}", final_state);
        },
        10,
    ) {
        Ok(_) => println!("Completed successfully."),
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn run(
    child: &mut std::process::Child,
    team: logic::Team,
    input: logic::RobotInput,
) -> serde_json::Result<logic::RobotOutput> {
    let actions = input.state.teams[&team]
        .iter()
        .map(|id| {
            // serde_json::to_writer(child.stdin.as_mut().expect("Failed to open stdin"), &input)?;
            child
                .stdin
                .as_mut()
                .expect("Failed to open stdin")
                .write(&serde_json::to_vec(&input)?)
                .unwrap();
            let action = serde_json::from_reader(child.stdout.as_mut().unwrap())?;
            Ok((*id, action))
        })
        .collect::<Result<_, _>>()?;

    Ok(logic::RobotOutput { actions })
}
