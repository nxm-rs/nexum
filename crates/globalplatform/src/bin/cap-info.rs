//! Utility to display information about CAP files without installing them
//!
//! This binary provides a simple tool to analyze CAP files and display their
//! package AIDs, applet AIDs, and other metadata.

use apdu_globalplatform::load::LoadCommandStream;
use clap::Parser;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the CAP file to analyze
    cap_file: PathBuf,

    /// Show detailed information about CAP file contents
    #[arg(short, long)]
    verbose: bool,

    /// Dump contents of a specific file in the CAP archive
    #[arg(long)]
    dump_file: Option<String>,

    /// Show very detailed debugging information
    #[arg(short, long)]
    debug: bool,

    /// Interactive mode to select applets for installation
    #[arg(short, long)]
    interactive: bool,
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

    // If requested, dump a specific file from the CAP
    if let Some(file_to_dump) = &cli.dump_file {
        let file = std::fs::File::open(&cli.cap_file)?;
        let mut zip = zip::ZipArchive::new(file)?;

        // Try to find the file (exact match or matching end)
        let mut found = false;
        for i in 0..zip.len() {
            let file_name = zip.by_index(i)?.name().to_string();
            if file_name == *file_to_dump || file_name.ends_with(file_to_dump) {
                println!("Contents of {}:", file_name);
                let mut contents = Vec::new();
                zip.by_name(&file_name)?.read_to_end(&mut contents)?;

                // Print hex dump
                for (i, chunk) in contents.chunks(16).enumerate() {
                    print!("{:08x}  ", i * 16);
                    for b in chunk {
                        print!("{:02x} ", b);
                    }

                    // Padding for last line if needed
                    for _ in 0..(16 - chunk.len()) {
                        print!("   ");
                    }

                    // Print ASCII representation
                    print!("  ");
                    for b in chunk {
                        if *b >= 32 && *b <= 126 {
                            print!("{}", *b as char);
                        } else {
                            print!(".");
                        }
                    }
                    println!();
                }
                found = true;
                break;
            }
        }

        if !found {
            println!("File not found: {}", file_to_dump);
            println!("\nAvailable files:");
            for i in 0..zip.len() {
                println!("  {}", zip.by_index(i)?.name());
            }
        }

        return Ok(());
    }

    // List all files in the CAP if in debug mode
    if cli.debug {
        let file = std::fs::File::open(&cli.cap_file)?;
        let mut zip = zip::ZipArchive::new(file)?;

        println!("CAP archive contains {} files:", zip.len());
        for i in 0..zip.len() {
            let file = zip.by_index(i)?;
            println!("  {}: {} bytes", file.name(), file.size());
        }
        println!();
    }

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
        for i in 0..info.applet_aids.len() {
            let aid = &info.applet_aids[i];
            let name = if i < info.applet_names.len() {
                &info.applet_names[i]
            } else {
                "Unknown"
            };
            println!("  {}. {} - AID: {}", i + 1, name, hex::encode_upper(aid));
        }
    }

    // Display detailed information if requested
    if cli.verbose {
        println!("\nCAP File Contents:");
        for (i, file) in info.files.iter().enumerate() {
            println!("  {}. {}", i + 1, file);
        }
    }

    // Interactive mode for selecting applets
    if cli.interactive && !info.applet_aids.is_empty() {
        println!("\nInteractive Installation Mode");
        println!("=============================");

        println!("Select applets to install:");
        println!("  0. All applets");
        for i in 0..info.applet_aids.len() {
            let name = if i < info.applet_names.len() {
                &info.applet_names[i]
            } else {
                "Unknown"
            };
            println!("  {}. {}", i + 1, name);
        }

        print!("\nEnter selection (0-{}): ", info.applet_aids.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let selection = input.trim().parse::<usize>().unwrap_or(0);

        if selection == 0 {
            println!("Installing all applets...");
            // Here you would call the installation code for all applets
            for i in 0..info.applet_aids.len() {
                let name = if i < info.applet_names.len() {
                    &info.applet_names[i]
                } else {
                    "Unknown"
                };
                println!("  Installing: {}", name);
                // Call installation code for this specific applet
            }
        } else if selection <= info.applet_aids.len() {
            let index = selection - 1;
            let name = if index < info.applet_names.len() {
                &info.applet_names[index]
            } else {
                "Unknown"
            };
            println!("Installing applet: {}", name);
            // Call installation code for this specific applet
        } else {
            println!("Invalid selection!");
        }
    }

    Ok(())
}
