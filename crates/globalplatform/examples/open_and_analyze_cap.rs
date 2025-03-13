//! Example to open and analyze a CAP file
//!
//! This example opens a CAP file, extracts information, and displays it
//! without loading it to a card.

use std::path::PathBuf;

use nexum_apdu_globalplatform::load::LoadCommandStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <cap_file_path>", args[0]);
        return Ok(());
    }

    let cap_file_path = PathBuf::from(&args[1]);
    if !cap_file_path.exists() {
        println!("CAP file not found: {:?}", cap_file_path);
        return Ok(());
    }

    println!("Analyzing CAP file: {:?}", cap_file_path);

    // Open the CAP file
    let file = std::fs::File::open(&cap_file_path)?;

    // Create the load command stream
    let stream = match LoadCommandStream::from_file_handle(file) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open CAP file: {:?}", e);
            return Ok(());
        }
    };

    // Display information about the CAP file
    println!("CAP file successfully opened.");
    println!("Total blocks: {}", stream.blocks_count());

    // If we want to actually examine the zip contents
    let zip_file = std::fs::File::open(&cap_file_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    println!("\nCAP file contents:");
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        println!("  {:<20} - {} bytes", file.name(), file.size());
    }

    // Look for the applet.cap or similar to try to extract the AIDs
    // This is just a simple example - a real implementation would parse the CAP components properly
    if let Ok(mut file) = archive.by_name("applet.cap") {
        use std::io::Read;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        println!("\nPossible AIDs found in applet.cap:");
        find_possible_aids(&contents);
    } else if let Ok(mut file) = archive.by_name("Header.cap") {
        use std::io::Read;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        println!("\nFound Header.cap - Extract information:");
        // Very basic parsing - real implementation would parse according to the spec
        if contents.len() > 10 {
            println!("  CAP file version: {}.{}", contents[1], contents[2]);
            let flags = contents[3];
            println!("  Flags: {:#04X}", flags);

            // Try to find the package AID
            if contents.len() > 15 && contents[13] < 16 {
                // Assuming AID length field
                let aid_len = contents[13] as usize;
                if 14 + aid_len <= contents.len() {
                    let package_aid = &contents[14..14 + aid_len];
                    println!("  Package AID: {}", hex::encode_upper(package_aid));
                }
            }
        }
    }

    println!("\nAnalysis complete.");
    Ok(())
}

/// Simple function to find byte sequences that might be AIDs
fn find_possible_aids(data: &[u8]) {
    // AIDs typically start with 0xA0 and are usually 5-16 bytes
    // This is a very simple heuristic just for demonstration

    for i in 0..data.len() {
        if data[i] == 0xA0 && i + 5 <= data.len() {
            // Check if the prior byte might be a length field for BER-TLV
            if i > 0 && data[i - 1] > 4 && data[i - 1] < 17 {
                let len = data[i - 1] as usize;
                if i + len <= data.len() {
                    let possible_aid = &data[i..i + len];
                    println!("  Possible AID: {}", hex::encode_upper(possible_aid));
                }
            }
        }
    }
}
