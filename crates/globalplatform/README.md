# APDU GlobalPlatform

A Rust implementation of the GlobalPlatform Card Specification for smart card management.

## Overview

This crate provides functionality for managing smart cards that implement the GlobalPlatform specification,
including secure channel establishment, applet installation, and package loading.

The main entry point is the `GlobalPlatform` struct, which provides high-level methods
for common card management operations.

## Features

- Card manager selection
- SCP02 secure channel establishment
- Package loading
- Applet installation
- Application/package deletion
- Status listing

## Usage Example

```rust
use nexum_apdu_core::CardExecutor;
use nexum_apdu_globalplatform::GlobalPlatform;
use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a card
    let manager = PcscDeviceManager::new()?;
    let readers = manager.list_readers()?;
    let reader = readers.iter().find(|r| r.has_card()).unwrap();
    let transport = manager.open_reader(reader.name())?;
    let executor = CardExecutor::new(transport);

    // Create GlobalPlatform instance
    let mut gp = GlobalPlatform::new(executor);

    // Select the Card Manager
    gp.select_card_manager()?;

    // Open secure channel
    gp.open_secure_channel()?;

    // List applications
    let response = gp.get_applications_status()?;
    println!("Applications: {:?}", response);

    // Delete a package
    let package_aid = hex::decode("A0000000030000")?;
    gp.delete_object_and_related(&package_aid)?;

    // Load a CAP file
    gp.load_cap_file("path/to/package.cap", Some(Box::new(|current, total| {
        println!("Loading block {}/{}", current, total);
        Ok(())
    })))?;

    // Install an applet
    let package_aid = hex::decode("A0000000030000")?;
    let applet_aid = hex::decode("A000000003000001")?;
    let instance_aid = hex::decode("A000000003000001")?;
    gp.install_for_install_and_make_selectable(
        &package_aid, &applet_aid, &instance_aid, &[]
    )?;

    Ok(())
}
```

## Supported Standards

- GlobalPlatform Card Specification 2.1.1
- Secure Channel Protocol 02 (SCP02)

## Credits

This implementation is based on the GlobalPlatform Card Specification and draws inspiration from:

- [GlobalPlatformPro](https://github.com/martinpaljak/GlobalPlatformPro)
- [keycard-go](https://github.com/status-im/keycard-go)

## License

Licensed under the MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT).
