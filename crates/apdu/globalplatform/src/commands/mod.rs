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
pub use delete::DeleteCommand;
pub use external_authenticate::ExternalAuthenticateCommand;
pub use get_response::GetResponseCommand;
pub use get_status::GetStatusCommand;
pub use initialize_update::InitializeUpdateCommand;
pub use install::InstallCommand;
pub use load::LoadCommand;
pub use put_key::PutKeyCommand;
pub use select::SelectCommand;
pub use store_data::StoreDataCommand;
