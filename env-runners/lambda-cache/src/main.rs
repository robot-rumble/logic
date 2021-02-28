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
    let ext = wasmer_engine_jit::JITArtifact::get_default_extension(target.triple());
    assert_eq!(ext, "wjit");
    let tunables = wasmer::BaseTunables::for_target(&target);
    let compiler_config = wasmer_compiler_llvm::LLVM::new();
    let engine = wasmer_engine_jit::JIT::new(compiler_config)
        .target(target)
        .engine();
    for path in args {
        let bytes = std::fs::read(&path)?;
        let artifact = wasmer_engine_jit::JITArtifact::new(&engine, &bytes, &tunables)?;
        let mut artifact_path = cache_dir.join(path.file_name().unwrap());
        artifact_path.set_extension(ext);
        std::fs::write(artifact_path, artifact.serialize()?)?
    }

    Ok(())
}
