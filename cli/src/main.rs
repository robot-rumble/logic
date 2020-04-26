use anyhow::{anyhow, Context};
use itertools::Itertools;
use native_runner::TokioRunner;
use std::fs;
use tokio::process::Command;
use tokio::{io, task};
use wasi_runner::WasiProcess;

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        eprintln!("ERROR: {}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| eprintln!("because: {}", cause));
        std::process::exit(1);
    }
}

async fn try_main() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let command = args
        .next()
        .ok_or_else(|| anyhow!("you must pass a command to run"))?;
    let args = args.collect::<Vec<_>>();
    if command == "wasm" {
        let sourcedir = tempfile::tempdir().context("couldn't create temporary directory")?;
        let (wasm, source) = args
            .into_iter()
            .collect_tuple()
            .ok_or_else(|| anyhow!("you must pass a wasm file and a source code file to `wasm`"))?;
        let wasm = fs::read(wasm).context("couldn't read wasm source")?;
        eprintln!("compiling wasm");
        let module = wasmer_runtime::compile(&wasm).context("couldn't compile wasm module")?;
        eprintln!("done!");
        let sourcecode_path = sourcedir.path().join("sourcecode");
        fs::hard_link(&source, &sourcecode_path)
            .or_else(|_| fs::copy(source, sourcecode_path).map(drop))
            .context("couldn't copy file to tempdir")?;

        let version = wasmer_wasi::get_wasi_version(&module, false)
            .unwrap_or(wasmer_wasi::WasiVersion::Latest);

        let make_runner = || -> anyhow::Result<_> {
            let mut state = wasmer_wasi::state::WasiState::new("robot");
            wasi_runner::add_stdio(&mut state);
            state
                .preopen(|p| p.directory(sourcedir.path()).alias("source").read(true))
                .unwrap()
                .arg("/source/sourcecode");
            let imports =
                wasmer_wasi::generate_import_object_from_state(state.build().unwrap(), version);
            let instance = module
                .instantiate(&imports)
                .map_err(|e| anyhow!("error instantiating module: {}", e))?;
            let mut proc = WasiProcess::spawn(instance);
            let stdin = io::BufWriter::new(proc.take_stdin().unwrap());
            let stdout = io::BufReader::new(proc.take_stdout().unwrap());
            task::spawn(proc);
            Ok(TokioRunner::new(stdin, stdout))
        };
        eprintln!("initializing runners");
        let (r1, r2) = tokio::join!(make_runner()?, make_runner()?);
        eprintln!("done!");
        run(r1, r2).await
    } else {
        let mut c1 = Command::new(&command);
        c1.args(&args);
        let mut c2 = Command::new(command);
        c2.args(args);
        let (r1, r2) = tokio::join!(TokioRunner::new_cmd(c1), TokioRunner::new_cmd(c2));
        run(r1, r2).await
    }
    Ok(())
}

async fn run<R: logic::RobotRunner>(r1: logic::ProgramResult<R>, r2: logic::ProgramResult<R>) {
    let output = logic::run(r1, r2, turn_cb, 10).await;
    println!("Output: {:?}", output);
}

fn turn_cb(turn_state: &logic::CallbackInput) {
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
}
