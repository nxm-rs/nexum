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
                "secure" => {
                    // Provide a helpful error for users of the old API
                    return Err(syn::Error::new(
                        key.span(),
                        "The 'secure' field is no longer supported. Use 'required_security_level' instead, e.g., required_security_level: SecurityLevel::none()",
                    ));
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
    response_name: &Ident,
    result_name: &Ident,
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
        #vis struct #command_name {
            p1: u8,
            p2: u8,
            data: Option<bytes::Bytes>,
            le: Option<ExpectedLength>,
        }

        impl #command_name {
            /// Create a new command with given P1 and P2 parameters
            pub const fn new(p1: u8, p2: u8) -> Self {
                Self {
                    p1,
                    p2,
                    data: None,
                    le: None,
                }
            }

            /// Add data to the command
            pub fn with_data(mut self, data: impl Into<bytes::Bytes>) -> Self {
                self.data = Some(data.into());
                self
            }

            /// Set the expected length
            pub const fn with_le(mut self, le: ExpectedLength) -> Self {
                self.le = Some(le);
                self
            }

            // Builder methods
            #(#builder_methods)*
        }

        impl nexum_apdu_core::ApduCommand for #command_name {
            type Response = #response_name;
            type ResultType = #result_name;

            fn class(&self) -> u8 {
                #cla
            }

            fn instruction(&self) -> u8 {
                #ins
            }

            fn p1(&self) -> u8 {
                self.p1
            }

            fn p2(&self) -> u8 {
                self.p2
            }

            fn data(&self) -> Option<&[u8]> {
                self.data.as_deref()
            }

            fn expected_length(&self) -> Option<ExpectedLength> {
                self.le
            }

            fn required_security_level(&self) -> SecurityLevel {
                #required_security_level
            }
        }
    };

    Ok(tokens)
}
