use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn main() {
    ensure_tool_installed("wasm-pack", "wasm-pack", "0.10.3");
    ensure_tool_installed("wasm-opt", "wasm-opt", "v0.116.1");

    // List of sub-crates to build with wasm-pack
    let subcrates = ["crates/injector", "crates/injected", "crates/worker"];
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let dist_dir = current_dir.join("dist");

    // Create the dist directory if it doesn’t exist
    if !dist_dir.exists() {
        fs::create_dir(&dist_dir).expect("Failed to create dist directory");
    }

    // Build each sub-crate with wasm-pack and move the pkg output to dist
    for crate_path in &subcrates {
        let crate_name = Path::new(crate_path)
            .file_name()
            .expect("Invalid crate path")
            .to_str()
            .expect("Invalid crate name");

        println!("Building {} with wasm-pack...", crate_name);

        let status = Command::new("wasm-pack")
            .arg("build")
            .arg("--release")
            .arg("--target")
            .arg("web")
            .arg(crate_path)
            .status()
            .expect("Failed to execute wasm-pack");

        if !status.success() {
            panic!("wasm-pack build failed for {}", crate_name);
        }

        let pkg_dir = current_dir.join(crate_path).join("pkg");
        let target_dir = dist_dir.join(crate_name);
        
        if !pkg_dir.exists() {
            panic!(
                "Expected pkg directory for {} not found at {:?}",
                crate_name, pkg_dir
            );
        }

        if target_dir.exists() {
            fs::remove_dir_all(&target_dir).expect("Failed to clean target directory");
        }
        fs::rename(&pkg_dir, &target_dir).expect("Failed to move pkg to dist directory");
    }

    // Copy contents of the public folder into dist
    let public_dir = current_dir.join("public");
    if public_dir.exists() {
        copy_dir_recursive(&public_dir, &dist_dir).expect("Failed to copy public assets to dist");
    }

    // Tell Cargo to re-run this build script if any sub-crate or public file changes
    for crate_path in &subcrates {
        println!("cargo:rerun-if-changed={}", crate_path);
    }
    println!("cargo:rerun-if-changed=public");
}

// Helper function to copy files from one directory to another recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());

        if path.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

// Helper function to check if a tool is installed
fn ensure_tool_installed(tool: &str, crate_name: &str, version: &str) {
    if Command::new(tool).arg("--version").output().is_err() {
        println!("{} not found, installing...", tool);
        install_tool(crate_name, version);
    }
}

// Function to install a tool using cargo install
fn install_tool(crate_name: &str, version: &str) {
    let status = Command::new("cargo")
        .arg("install")
        .arg(crate_name)
        .arg("--version")
        .arg(version)
        .status()
        .expect("Failed to run cargo install");

    if !status.success() {
        eprintln!("Failed to install {}", crate_name);
        exit(1);
    }
}
