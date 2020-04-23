use itertools::Itertools;
use native_runner::CliRunner;

#[tokio::main]
async fn main() {
    let make_cmd = || {
        let mut args = std::env::args_os();
        let command = args.nth(1).expect("You must pass a command to run");
        let mut cmd = tokio::process::Command::new(command);
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
