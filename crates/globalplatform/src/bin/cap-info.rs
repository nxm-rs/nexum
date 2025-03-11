//! Utility to display information about CAP files without installing them
//!
//! This binary provides a simple tool to analyze CAP files and display their
//! package AIDs, applet AIDs, and other metadata.

use apdu_globalplatform::load::{CapFileInfo, LoadCommandStream};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the CAP file to analyze
    cap_file: PathBuf,

    /// Show detailed information about CAP file contents
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Check if CAP file exists
    if !cli.cap_file.exists() {
        return Err(format!("CAP file not found: {}", cli.cap_file.display()).into());
    }

    println!("Analyzing CAP file: {}", cli.cap_file.display());
    println!("========================================");

    // Extract CAP file information
    let info = LoadCommandStream::extract_info(&cli.cap_file)?;

    // Display package AID
    if let Some(package_aid) = &info.package_aid {
        println!("Package AID: {}", hex::encode_upper(package_aid));
    } else {
        println!("Package AID: Not found");
    }

    // Display version if available
    if let Some((major, minor)) = info.version {
        println!("Version: {}.{}", major, minor);
    } else {
        println!("Version: Unknown");
    }

    // Display applet AIDs
    println!("\nApplets:");
    if info.applet_aids.is_empty() {
        println!("  None found");
    } else {
        for (i, aid) in info.applet_aids.iter().enumerate() {
            println!("  {}. AID: {}", i + 1, hex::encode_upper(aid));
        }
    }

    // Display detailed information if requested
    if cli.verbose {
        println!("\nCAP File Contents:");
        for (i, file) in info.files.iter().enumerate() {
            println!("  {}. {}", i + 1, file);
        }
    }

    Ok(())
}
