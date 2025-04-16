//! Command parsing and expansion logic

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Ident, ItemFn, Token, Visibility, braced, parse::ParseStream};

/// Command definition parsed from the `command` block
pub(crate) struct CommandDef {
    /// Class byte (CLA)
    pub cla: Expr,
    /// Instruction byte (INS)
    pub ins: Expr,
    /// Required security level for the command
    pub required_security_level: Option<Expr>,
    /// Builder methods
    pub builders: Vec<ItemFn>,
}

impl CommandDef {
    /// Parse a command definition from a ParseStream
    pub(crate) fn parse<'a>(input: &'a ParseStream<'a>) -> syn::Result<Self> {
        let mut cla = None;
        let mut ins = None;
        let mut required_security_level = None;
        let mut builders = Vec::new();

        // Parse each field in the command block
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let key_str = key.to_string();

            match key_str.as_str() {
                "cla" => {
                    input.parse::<Token![:]>()?;
                    cla = Some(input.parse()?);
                    input.parse::<Token![,]>()?;
                }
                "ins" => {
                    input.parse::<Token![:]>()?;
                    ins = Some(input.parse()?);
                    input.parse::<Token![,]>()?;
                }
                "required_security_level" => {
                    input.parse::<Token![:]>()?;
                    required_security_level = Some(input.parse()?);
                    input.parse::<Token![,]>()?;
                }
                "builders" => {
                    let content;
                    braced!(content in input);

                    // Parse builder methods
                    while !content.is_empty() {
                        let fn_item: ItemFn = content.parse()?;
                        builders.push(fn_item);
                    }

                    if !input.is_empty() {
                        input.parse::<Token![,]>()?;
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown command field: {}", key),
                    ));
                }
            }
        }

        // Ensure required fields are present
        let cla =
            cla.ok_or_else(|| syn::Error::new(Span::call_site(), "Missing CLA field in command"))?;
        let ins =
            ins.ok_or_else(|| syn::Error::new(Span::call_site(), "Missing INS field in command"))?;

        Ok(Self {
            cla,
            ins,
            required_security_level,
            builders,
        })
    }
}

/// Expand a command definition into a command struct
pub(crate) fn expand_command(
    command: &CommandDef,
    vis: &Visibility,
    command_name: &Ident,
    ok_name: &Ident,
    error_name: &Ident,
    parse_impl: &TokenStream,
) -> Result<TokenStream, syn::Error> {
    let cla = &command.cla;
    let ins = &command.ins;

    // Use the provided security level or default to SecurityLevel::none()
    let required_security_level = command
        .required_security_level
        .as_ref()
        .map_or_else(|| quote! { SecurityLevel::none() }, |expr| quote! { #expr });

    // Generate builder methods
    let builder_methods = &command.builders;

    let tokens = quote! {
        /// APDU command implementation
        ///
        /// This struct represents an APDU command that can be sent to a smart card.
        /// It encapsulates all the parameters required for the command, including
        /// class (CLA), instruction (INS), parameters (P1, P2), command data, and
        /// expected response length (Le).
        #vis struct #command_name {
            /// Parameter 1 (P1) - specific meaning depends on the command
            p1: u8,
            /// Parameter 2 (P2) - specific meaning depends on the command
            p2: u8,
            /// Optional command data
            data: Option<bytes::Bytes>,
            /// Expected response length
            le: Option<ExpectedLength>,
        }

        impl #command_name {
            /// Create a new command with given P1 and P2 parameters
            ///
            /// This is the basic constructor that initializes a command with the
            /// specified P1 and P2 values. Data and expected length can be added
            /// using the builder methods.
            pub const fn new(p1: u8, p2: u8) -> Self {
                Self {
                    p1,
                    p2,
                    data: None,
                    le: None,
                }
            }

            /// Add data to the command
            ///
            /// Sets the command data field (Lc + data). This is typically used
            /// to provide parameters or input data for the command.
            pub fn with_data(mut self, data: impl Into<bytes::Bytes>) -> Self {
                self.data = Some(data.into());
                self
            }

            /// Set the expected length
            ///
            /// Specifies the maximum number of bytes expected in the response (Le).
            /// This tells the card how much data the terminal expects to receive.
            pub const fn with_le(mut self, le: ExpectedLength) -> Self {
                self.le = Some(le);
                self
            }

            // Builder methods
            #(#builder_methods)*
        }

        impl nexum_apdu_core::ApduCommand for #command_name {
            /// The success response type for this command
            type Success = #ok_name;
            
            /// The error response type for this command
            type Error = #error_name;
            
            /// Converts a general APDU error into a command-specific error
            fn convert_error(error: nexum_apdu_core::Error) -> Self::Error {
                Self::Error::ResponseError(error)
            }

            /// Returns the command class (CLA) byte
            fn class(&self) -> u8 {
                #cla
            }

            /// Returns the instruction (INS) byte
            fn instruction(&self) -> u8 {
                #ins
            }

            /// Returns the first parameter (P1) byte
            fn p1(&self) -> u8 {
                self.p1
            }

            /// Returns the second parameter (P2) byte
            fn p2(&self) -> u8 {
                self.p2
            }

            /// Returns the command data field if present
            fn data(&self) -> Option<&[u8]> {
                self.data.as_deref()
            }

            /// Returns the expected response length (Le) if specified
            fn expected_length(&self) -> Option<ExpectedLength> {
                self.le
            }

            /// Returns the security level required for this command
            fn required_security_level(&self) -> SecurityLevel {
                #required_security_level
            }

            /// Parses a raw APDU response into a command-specific success or error type
            ///
            /// This method interprets the status word (SW1-SW2) and response data
            /// according to the command specification, returning either a success
            /// variant or an appropriate error variant.
            #parse_impl
        }
    };

    Ok(tokens)
}
