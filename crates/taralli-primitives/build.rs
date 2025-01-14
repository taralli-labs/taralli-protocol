use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=scripts/check_and_update_abi.sh");
    println!("cargo:rerun-if-env-changed=CARGO_MANIFEST_DIR");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_NAME");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let package_name = env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set");

    let script_path = PathBuf::from(&manifest_dir)
        .join("scripts")
        .join("check_and_update_abi.sh");

    // Set the CRATE_DIR environment variable for the script
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(script_path.to_str().unwrap())
        .current_dir(&manifest_dir)
        .env("CRATE_DIR", &manifest_dir)
        .env("PACKAGE_NAME", &package_name);

    let status = command
        .status()
        .expect("Failed to execute check_and_update_abi.sh");

    if !status.success() {
        panic!("Failed to check and update ABI");
    }
}
