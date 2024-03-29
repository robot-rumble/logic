use std::error::Error;
use std::path::PathBuf;
use wasmer::{Artifact, EngineBuilder, LLVM};
use wasmer_compiler::ArtifactCreate;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().map(PathBuf::from).skip(1);
    let cache_dir = args.next().ok_or("must pass a cache dir")?;

    std::fs::create_dir_all(&cache_dir)?;

    let target = wasmer::Target::new(
        "x86_64-unknown-linux-musl".parse().unwrap(),
        Default::default(),
    );
    let tunables = wasmer::BaseTunables::for_target(&target);
    let engine = EngineBuilder::new(LLVM::default()).engine();
    for path in args {
        let bytes = std::fs::read(&path)?;
        let artifact = Artifact::new(&engine, &bytes, &tunables)?;
        let mut artifact_path = cache_dir.join(path.file_name().unwrap());
        artifact_path.set_extension("wasmu");
        std::fs::write(artifact_path, artifact.serialize()?)?
    }

    Ok(())
}
