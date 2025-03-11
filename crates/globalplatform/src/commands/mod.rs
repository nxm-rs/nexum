//! GlobalPlatform command definitions
//!
//! This module contains the definitions of all GlobalPlatform commands
//! using the apdu-macros system.

// Submodules
pub mod delete;
pub mod external_authenticate;
pub mod get_response;
pub mod get_status;
pub mod initialize_update;
pub mod install;
pub mod load;
pub mod put_key;
pub mod select;
pub mod store_data;

// Re-exports for convenience
pub use delete::{DeleteCommand, DeleteResponse};
pub use external_authenticate::{ExternalAuthenticateCommand, ExternalAuthenticateResponse};
pub use get_response::{GetResponseCommand, GetResponseResponse};
pub use get_status::{ApplicationInfo, GetStatusCommand, GetStatusResponse, LoadFileInfo};
pub use initialize_update::{InitializeUpdateCommand, InitializeUpdateResponse};
pub use install::{InstallCommand, InstallResponse};
pub use load::{LoadCommand, LoadResponse};
pub use put_key::{PutKeyCommand, PutKeyResponse};
pub use select::{SelectCommand, SelectResponse};
pub use store_data::{StoreDataCommand, StoreDataResponse};
