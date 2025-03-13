//! GlobalPlatform implementation for smart card management
//!
//! This crate provides functionality for managing smart cards that implement
//! the GlobalPlatform specification, including secure channel establishment,
//! applet installation, and package loading.
//!
//! The main entry point is the `GlobalPlatform` struct, which provides
//! high-level methods for common card management operations.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

pub mod application;
pub mod commands;
pub mod constants;
pub mod crypto;
pub mod error;
pub mod load;
pub mod secure_channel;
pub mod session;
pub mod util;

// Re-exports
pub use application::GlobalPlatform;
pub use error::{Error, Result};
pub use load::CapFileInfo;
pub use secure_channel::GPSecureChannel;
pub use session::{Keys, Session};

// Re-export from nexum_apdu_core for convenience
pub use nexum_apdu_core::{ResponseAwareExecutor, SecureChannelExecutor};

// Export DefaultKeys for easy initialization
pub use application::DefaultKeys;

// Export main commands
pub use commands::{
    DeleteCommand, DeleteResponse, GetStatusCommand, GetStatusResponse, InitializeUpdateCommand,
    InitializeUpdateResponse, InstallCommand, InstallResponse, LoadCommand, LoadResponse,
    SelectCommand, SelectResponse,
};

/// Convenience functions for common operations
pub mod operations {
    use nexum_apdu_core::CardExecutor;
    use nexum_apdu_core::prelude::Executor;
    use nexum_apdu_core::{ResponseAwareExecutor, SecureChannelExecutor};

    use crate::{Error, GlobalPlatform, Result};

    /// Connect to a card, select the card manager, and establish a secure channel
    pub fn connect_and_setup<T>(
        executor: CardExecutor<T>,
    ) -> Result<GlobalPlatform<CardExecutor<T>>>
    where
        T: nexum_apdu_core::transport::CardTransport,
    {
        // Create GlobalPlatform instance
        let mut gp = GlobalPlatform::new(executor);

        // Select the Card Manager
        gp.select_card_manager()?;

        // Open secure channel with default keys
        gp.open_secure_channel()?;

        Ok(gp)
    }

    /// List all applications on the card
    pub fn list_applications<E>(
        gp: &mut GlobalPlatform<E>,
    ) -> Result<Vec<crate::commands::get_status::ApplicationInfo>>
    where
        E: Executor + ResponseAwareExecutor + SecureChannelExecutor,
    {
        let response = gp.get_applications_status()?;
        Ok(response.parse_applications())
    }

    /// List all packages on the card
    pub fn list_packages<E>(
        gp: &mut GlobalPlatform<E>,
    ) -> Result<Vec<crate::commands::get_status::LoadFileInfo>>
    where
        E: Executor + ResponseAwareExecutor + SecureChannelExecutor,
    {
        let response = gp.get_load_files_status()?;
        Ok(response.parse_load_files())
    }

    /// Delete a package and all of its applications
    pub fn delete_package<E>(gp: &mut GlobalPlatform<E>, aid: &[u8]) -> Result<()>
    where
        E: Executor + ResponseAwareExecutor + SecureChannelExecutor,
    {
        // Delete the package and all related applications
        let response = gp.delete_object_and_related(aid)?;

        // Check if deletion was successful
        if !matches!(response, crate::commands::DeleteResponse::Success) {
            return Err(Error::Other("Failed to delete package"));
        }

        Ok(())
    }

    /// Install a CAP file on the card
    pub fn install_cap_file<E, P: AsRef<std::path::Path>>(
        gp: &mut GlobalPlatform<E>,
        cap_path: P,
        make_selectable: bool,
        install_params: &[u8],
    ) -> Result<()>
    where
        E: Executor + ResponseAwareExecutor + SecureChannelExecutor,
    {
        // First analyze the CAP file to extract package and applet AIDs
        let cap_info = gp.analyze_cap_file(&cap_path)?;

        let package_aid = cap_info
            .package_aid
            .ok_or(Error::CapFile("Missing package AID"))?;

        // Install for load
        gp.install_for_load(&package_aid, None)?;

        // Load the CAP file
        gp.load_cap_file(&cap_path, None)?;

        // If requested, install and make selectable for each applet
        if make_selectable && !cap_info.applet_aids.is_empty() {
            for applet_aid in &cap_info.applet_aids {
                // Use the same AID for instance
                gp.install_for_install_and_make_selectable(
                    &package_aid,
                    applet_aid,
                    applet_aid,
                    install_params,
                )?;
            }
        }

        Ok(())
    }
}
