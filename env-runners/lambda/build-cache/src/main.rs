use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[cfg(not(target_arch = "x86_64"))]
compile_error!(
    "can't build wasmer caches for the x86_64-unknown-linux-musl target on a non-x86_64 platform"
);

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().map(PathBuf::from);
    args.next();
    let cache_dir = args.next().ok_or("must pass a cache dir")?;

    std::fs::create_dir_all(&cache_dir)?;

    for path in args {
        let bytes = std::fs::read(&path)?;
        let module = wasmer_runtime::compile(&bytes)?;
        std::fs::write(
            cache_dir.join(path.file_name().unwrap()),
            module
                .cache()
                .and_then(|artifact| artifact.serialize())
                .map_err(CacheError)?,
        )?
    }

    Ok(())
}

#[derive(Debug)]
struct CacheError(wasmer_runtime::error::CacheError);
impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl Error for CacheError {}
