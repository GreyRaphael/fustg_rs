// build.rs
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    // 1. Tell Cargo when to re-run this script…
    println!("cargo:rerun-if-changed=config/fees.1st.toml");

    // 2. Locate the source file…
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_file = manifest_dir.join("config/fees.1st.toml");

    // 3. Compute the *final* destination directory:
    //
    //    By default, Cargo puts build‐script outputs in
    //    $CARGO_MANIFEST_DIR/target/{debug,release}/build/<pkg-hash>/out,
    //    but binaries themselves go in target/{debug,release}/.
    //
    //    If you really want the file *next to* your compiled binary
    //    (e.g. `target/debug/your_crate`), you can instead do:
    let profile = env::var("PROFILE").unwrap(); // “debug” or “release”
    let target_dir = manifest_dir.join("target").join(&profile).join("config");

    // 4. Make sure the target directory exists…
    fs::create_dir_all(&target_dir).unwrap();

    // 5. Copy the file over:
    let dest = target_dir.join("fees.1st.toml");
    fs::copy(&src_file, &dest).unwrap_or_else(|e| panic!("Failed to copy {:?} to {:?}: {}", src_file, dest, e));
}
