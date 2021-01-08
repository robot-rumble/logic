use std::error::Error;
use std::path::PathBuf;
use wasmer_engine::Artifact;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().map(PathBuf::from).skip(1);
    let cache_dir = args.next().ok_or("must pass a cache dir")?;

    std::fs::create_dir_all(&cache_dir)?;

    let target = wasmer::Target::new(
        "x86_64-unknown-linux-musl".parse().unwrap(),
        Default::default(),
    );
    let tunables = wasmer::BaseTunables::for_target(&target);
    let compiler_config = wasmer_compiler_llvm::LLVM::new();
    let engine = wasmer_engine_native::Native::new(compiler_config)
        .target(target)
        .engine();
    for path in args {
        let bytes = std::fs::read(&path)?;
        let artifact = wasmer_engine_native::NativeArtifact::new(&engine, &bytes, &tunables)?;
        std::fs::write(
            cache_dir.join(path.file_name().unwrap()),
            artifact.serialize()?,
        )?
    }

    Ok(())
}
