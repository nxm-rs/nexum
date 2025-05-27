use clap::{Args, Subcommand};
use std::error::Error;
use std::path::PathBuf;

use cipher::Key;
use nexum_apdu_core::prelude::*;
use nexum_apdu_globalplatform::{
    DefaultGlobalPlatform, GPSecureChannel, load::LoadCommandStream, session::Keys,
};
use nexum_apdu_transport_pcsc::PcscTransport;

// Constants for keycard package identification
/// Keycard development key
pub const KEYCARD_DEVELOPMENT_KEY: [u8; 16] = [
    0xc2, 0x12, 0xe0, 0x73, 0xff, 0x8b, 0x4b, 0xbf, 0xaf, 0xf4, 0xde, 0x8a, 0xb6, 0x55, 0x22, 0x1f,
];

/// Keycard package AID
pub const PACKAGE_AID: [u8; 7] = [0xA0, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01];

/// Keycard applet AID
pub const KEYCARD_AID: [u8; 8] = [0xA0, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x01];

/// NDEF applet AID
pub const NDEF_AID: [u8; 8] = [0xA0, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x02];

/// NDEF instance AID
pub const NDEF_INSTANCE_AID: [u8; 7] = [0xD2, 0x76, 0x00, 0x00, 0x85, 0x01, 0x01];

/// Cash applet AID
pub const CASH_AID: [u8; 8] = [0xA0, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x03];

/// Cash instance AID
pub const CASH_INSTANCE_AID: [u8; 9] = [0xA0, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x03, 0x01];

/// IDENT AID
pub const IDENT_AID: [u8; 8] = [0xA0, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x04];

/// IDENT instance AID
pub const IDENT_INSTANCE_AID: [u8; 9] = [0xA0, 0x00, 0x00, 0x08, 0x04, 0x00, 0x01, 0x04, 0x01];

/// Default keycard instance index
pub const KEYCARD_DEFAULT_INSTANCE_INDEX: u8 = 1;

/// Applet management commands
#[derive(Subcommand)]
pub enum AppletCommands {
    /// Install a CAP file to the card
    Install(InstallArgs),

    /// Delete the Keycard package from the card
    Delete(DeleteArgs),
}

/// Arguments for the Install command
#[derive(Args)]
pub struct InstallArgs {
    /// Path to the CAP file
    #[arg(short, long, required = true)]
    pub cap_file: PathBuf,

    /// Use Global Platform default keys instead of Keycard development keys
    #[arg(short, long)]
    pub use_default_key: bool,

    /// Instance index for the Keycard applet (1-255)
    #[arg(short, long, value_parser = clap::builder::ValueParser::new(|s: &str| -> Result<u8, String> {
        let val = s.parse::<u8>().map_err(|_| "Not a valid number".to_string())?;
        if val < 1 {
            Err("Instance index must be between 1 and 255".to_string())
        } else {
            Ok(val)
        }
    }))]
    pub instance_index: Option<u8>,
}

/// Arguments for the Delete command
#[derive(Args)]
pub struct DeleteArgs {
    /// Use Global Platform default keys instead of Keycard development keys
    #[arg(short, long)]
    pub use_default_key: bool,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub force: bool,
}

/// Creates an instance AID for the keycard applet with the given index
fn keycard_instance_aid(index: u8) -> [u8; 9] {
    if index < 1 {
        panic!("Instance index must be between 1 and 255");
    }

    let mut instance_aid = [0u8; 9];
    instance_aid[..8].copy_from_slice(&KEYCARD_AID);
    instance_aid[8] = index;
    instance_aid
}

/// Connect to the GlobalPlatform environment with appropriate keys
fn connect_globalplatform(
    transport: PcscTransport,
    use_default_key: bool,
) -> Result<
    nexum_apdu_globalplatform::GlobalPlatform<CardExecutor<GPSecureChannel<PcscTransport>>>,
    Box<dyn Error>,
> {
    // Create a secure channel based on the requested key type
    println!(
        "{}",
        if use_default_key {
            "Using Global Platform default keys"
        } else {
            "Using Keycard development keys"
        }
    );

    // Get reader name
    let reader_name = transport.reader_name().to_string();

    // Create keys based on option
    let gp = if use_default_key {
        // Use default keys (via DefaultGlobalPlatform)
        DefaultGlobalPlatform::connect(&reader_name)
            .map_err(|e| format!("Failed to connect to the reader: {}", e))?
    } else {
        // Use Keycard development keys
        // Create a key type that the SCP02 protocol can use
        let dev_key =
            Key::<nexum_apdu_globalplatform::crypto::Scp02>::from_slice(&KEYCARD_DEVELOPMENT_KEY);
        let keys = Keys::from_single_key(*dev_key);

        // Create new transport with config and wrap it in the secure channel
        let config = nexum_apdu_transport_pcsc::PcscConfig::default();
        let manager = nexum_apdu_transport_pcsc::PcscDeviceManager::new()
            .map_err(|e| format!("Failed to create PCSC device manager: {}", e))?;
        let transport = manager
            .open_reader_with_config(&reader_name, config)
            .map_err(|e| format!("Failed to open reader: {}", e))?;

        // Create secure channel with custom keys
        let secure_channel = GPSecureChannel::new(transport, keys);

        // Create executor with secure channel
        let executor = CardExecutor::new(secure_channel);

        // Create GlobalPlatform instance
        nexum_apdu_globalplatform::GlobalPlatform::new(executor)
    };

    Ok(gp)
}

/// Initialize the secure channel with the card
fn init_secure_channel<E>(
    gp: &mut nexum_apdu_globalplatform::GlobalPlatform<E>,
) -> Result<(), Box<dyn Error>>
where
    E: Executor + ResponseAwareExecutor + SecureChannelExecutor,
{
    // Select the Card Manager
    match gp.select_card_manager() {
        Ok(_) => println!("Card Manager selected successfully."),
        Err(e) => return Err(format!("Failed to select Card Manager: {}", e).into()),
    }

    // Open secure channel
    match gp.open_secure_channel() {
        Ok(_) => println!("Secure channel established."),
        Err(e) => return Err(format!("Failed to open secure channel: {}", e).into()),
    }

    Ok(())
}

/// Handle the applet command
pub fn applet_command(
    transport: PcscTransport,
    cmd: &AppletCommands,
) -> Result<(), Box<dyn Error>> {
    match cmd {
        AppletCommands::Install(args) => install_command(
            transport,
            &args.cap_file,
            args.use_default_key,
            args.instance_index,
        ),
        AppletCommands::Delete(args) => delete_command(transport, args),
    }
}

/// Install the keycard applet onto the card
fn install_command(
    transport: PcscTransport,
    cap_file: &PathBuf,
    use_default_key: bool,
    instance_index: Option<u8>,
) -> Result<(), Box<dyn Error>> {
    println!("Installing keycard from CAP file: {}", cap_file.display());

    if !cap_file.exists() {
        return Err("CAP file not found".into());
    }

    // Connect to the GlobalPlatform environment
    let mut gp = connect_globalplatform(transport, use_default_key)?;

    // Initialize secure channel
    init_secure_channel(&mut gp)?;

    // Analyze the CAP file
    println!("Analyzing CAP file...");
    let info = match LoadCommandStream::extract_info(cap_file) {
        Ok(info) => info,
        Err(e) => return Err(format!("Failed to extract CAP file info: {}", e).into()),
    };

    // Verify package AID
    let package_aid = match &info.package_aid {
        Some(aid) => {
            let aid_matches = aid.len() == PACKAGE_AID.len()
                && aid.iter().zip(PACKAGE_AID.iter()).all(|(a, b)| a == b);

            if !aid_matches {
                println!("Package AID does not match expected Keycard AID");
                println!(
                    "Will attempt to install anyway, but this might not be a valid Keycard package"
                );
            } else {
                println!("Verified package AID");
            }
            aid
        }
        None => return Err("Package AID not found in CAP file".into()),
    };

    // First try to delete any existing package with the same AID
    println!("Checking for existing package...");
    match gp.delete_object_and_related(package_aid) {
        Ok(_) => println!("Existing package deleted."),
        Err(_) => println!("No existing package found or not deletable."),
    }

    // Install for load
    println!("Installing for load...");
    match gp.install_for_load(package_aid, None) {
        Ok(_) => println!("Install for load successful."),
        Err(e) => return Err(format!("Install for load failed: {}", e).into()),
    }

    // Prepare callback for progress reporting
    let mut callback = |current: usize, total: usize| -> nexum_apdu_globalplatform::Result<()> {
        println!(
            "Loading block {}/{} ({}%)",
            current,
            total,
            (current * 100) / total
        );
        Ok(())
    };

    // Load the CAP file
    println!("Loading CAP file...");
    match gp.load_cap_file(cap_file, Some(&mut callback)) {
        Ok(_) => println!("CAP file loaded successfully."),
        Err(e) => return Err(format!("Failed to load CAP file: {}", e).into()),
    }

    // Determine instance index
    let index = instance_index.unwrap_or(KEYCARD_DEFAULT_INSTANCE_INDEX);
    let keycard_instance_aid = keycard_instance_aid(index);

    // Install applets with their respective instance AIDs
    // 1. Keycard applet
    println!("Installing Keycard applet...");
    match gp.install_for_install_and_make_selectable(
        package_aid,
        &KEYCARD_AID,
        &keycard_instance_aid,
        &[],
    ) {
        Ok(_) => println!("Keycard applet installed successfully."),
        Err(e) => println!("Failed to install Keycard applet: {}", e),
    }

    // 2. NDEF applet
    println!("Installing NDEF applet...");
    match gp
        .install_for_install_and_make_selectable(package_aid, &NDEF_AID, &NDEF_INSTANCE_AID, &[])
    {
        Ok(_) => println!("NDEF applet installed successfully."),
        Err(e) => println!("Failed to install NDEF applet: {}", e),
    }

    // 3. Cash applet
    println!("Installing Cash applet...");
    match gp
        .install_for_install_and_make_selectable(package_aid, &CASH_AID, &CASH_INSTANCE_AID, &[])
    {
        Ok(_) => println!("Cash applet installed successfully."),
        Err(e) => println!("Failed to install Cash applet: {}", e),
    }

    // 4. Ident applet
    println!("Installing Ident applet...");
    match gp.install_for_install_and_make_selectable(
        package_aid,
        &IDENT_AID,
        &IDENT_INSTANCE_AID,
        &[],
    ) {
        Ok(_) => println!("Ident applet installed successfully."),
        Err(e) => println!("Failed to install Ident applet: {}", e),
    }

    println!("Installation process completed.");
    Ok(())
}

/// Delete the Keycard package from the card
fn delete_command(transport: PcscTransport, args: &DeleteArgs) -> Result<(), Box<dyn Error>> {
    // Connect to the GlobalPlatform environment
    let mut gp = connect_globalplatform(transport, args.use_default_key)?;

    // Initialize secure channel
    init_secure_channel(&mut gp)?;

    // Get confirmation unless --force is specified
    let confirmed = args.force
        || crate::utils::session::default_confirmation(
            "Are you sure you want to delete the Keycard package? This operation cannot be undone.",
        );

    if confirmed {
        // Delete the Keycard package
        println!("Deleting Keycard package...");
        match gp.delete_object_and_related(&PACKAGE_AID) {
            Ok(_) => println!("Keycard package deleted successfully."),
            Err(e) => return Err(format!("Failed to delete Keycard package: {}", e).into()),
        }

        println!("Deletion process completed.");
    } else {
        println!("Operation cancelled.");
    }

    Ok(())
}
