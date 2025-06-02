//! Utility functions for macro expansion

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Lit, LitInt};

/// Create error tokens for compilation errors
pub(crate) fn error_tokens(message: &str, details: impl std::fmt::Display) -> TokenStream {
    let error_message = format!("{message}: {details}");
    quote! {
        compile_error!(#error_message);
    }
}

/// Create a byte literal
pub(crate) fn byte_lit(value: u8) -> Lit {
    Lit::Int(LitInt::new(&format!("0x{value:02X}u8"), Span::call_site()))
}
