//! Build script: stage the Snake SoC linker script into `OUT_DIR` and put that
//! directory on the linker's search path, so `-T link.ld` (set in
//! `.cargo/config.toml`) resolves without committing an absolute path.

use std::{env, fs, io, path::PathBuf};

fn main() -> io::Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"));

    // Copy the committed link.ld next to the build artifacts and expose the
    // directory so `link-arg=-Tlink.ld` finds it.
    fs::copy("link.ld", out_dir.join("link.ld"))?;
    println!("cargo:rustc-link-search={}", out_dir.display());

    // Re-link whenever the script changes.
    println!("cargo:rerun-if-changed=link.ld");
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
