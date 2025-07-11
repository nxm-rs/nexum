//! Procedural macros for working with APDU commands and responses
//!
//! This crate provides macros to simplify the definition of APDU commands
//! and responses according to ISO/IEC 7816-4 standards.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, Ident, Token, Visibility, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

mod command;
mod response;
mod utils;

use command::CommandDef;
use response::ResponseDef;
use utils::error_tokens;

/// Defines a paired APDU command and response
///
/// This macro creates both a command and its corresponding response type.
///
/// # Example
///
/// ```
/// use nexum_apdu_macros::apdu_pair;
/// use nexum_apdu_core::prelude::*;
///
/// apdu_pair! {
///     /// Select command for applications
///     pub struct Select {
///         command {
///             cla: 0x00,
///             ins: 0xA4,
///             required_security_level: SecurityLevel::none(),
///
///             builders {
///                 /// Select by AID
///                 pub fn by_aid(aid: impl Into<bytes::Bytes>) -> Self {
///                     Self::new(0x04, 0x00).with_data(aid.into()).with_le(0)
///                 }
///             }
///         }
///
///         response {
///             ok {
///                 #[sw(0x90, 0x00)]
///                 #[payload(field = "fci")]
///                 Selected {
///                     fci: Vec<u8>,
///                 }
///             }
///
///             errors {
///                 #[sw(0x6A, 0x82)]
///                 #[error("File not found")]
///                 NotFound,
///
///                 #[sw(_, _)]
///                 #[error("Other error: {sw1:02X}{sw2:02X}")]
///                 OtherError {
///                     sw1: u8,
///                     sw2: u8,
///                 }
///             }
///         }
///     }
/// }
/// ```
#[proc_macro]
pub fn apdu_pair(input: TokenStream) -> TokenStream {
    let pair = parse_macro_input!(input as ApduPair);

    match expand_apdu_pair(&pair) {
        Ok(expanded) => expanded.into(),
        Err(err) => err.into(),
    }
}

/// Definition of an APDU command and its response
struct ApduPair {
    vis: Visibility,
    struct_name: Ident,
    attrs: Vec<Attribute>,
    command: CommandDef,
    response: ResponseDef,
}

impl Parse for ApduPair {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // Parse attributes and visibility
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;

        // Parse struct keyword and name
        input.parse::<Token![struct]>()?;
        let struct_name = input.parse()?;

        // Parse the opening brace
        let content;
        braced!(content in input);

        // Parse command section
        content.parse::<Ident>()?; // 'command' keyword
        let command;
        braced!(command in content);
        let command_def = CommandDef::parse(&&command)?;

        // Parse response section
        content.parse::<Ident>()?; // 'response' keyword
        let response;
        braced!(response in content);
        let response_def = ResponseDef::parse(&&response)?;

        Ok(Self {
            vis,
            struct_name,
            attrs,
            command: command_def,
            response: response_def,
        })
    }
}

/// Expands an APDU pair into command and response definitions
fn expand_apdu_pair(pair: &ApduPair) -> Result<TokenStream2, TokenStream2> {
    // Generate names centrally
    let command_name = Ident::new(
        &format!("{}Command", pair.struct_name),
        pair.struct_name.span(),
    );

    let ok_name = Ident::new(&format!("{}Ok", pair.struct_name), pair.struct_name.span());

    let error_name = Ident::new(
        &format!("{}Error", pair.struct_name),
        pair.struct_name.span(),
    );

    // Module name
    let module_name = Ident::new(
        &pair.struct_name.to_string().to_snake_case(),
        pair.struct_name.span(),
    );

    // Get the response tokens and parse_impl
    let (response_tokens, parse_impl) =
        response::expand_response(&pair.response, &pair.vis, &ok_name, &error_name)
            .map_err(|e| error_tokens("Error expanding response", e))?;

    // Expand command with the parse_impl and type names
    let command_tokens = command::expand_command(
        &pair.command,
        &pair.vis,
        &command_name,
        &ok_name,
        &error_name,
        &parse_impl,
    )
    .map_err(|e| error_tokens("Error expanding command", e))?;

    let attrs = &pair.attrs;

    Ok(quote! {
        #(#attrs)*
        mod #module_name {
            use super::*;
            use nexum_apdu_core::prelude::*;

            #command_tokens

            #response_tokens
        }

        pub use #module_name::{#command_name, #ok_name, #error_name};
    })
}
