//! Response parsing and expansion logic

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Expr, ExprClosure, ExprLit, Field, Ident, ItemFn, Lit, Token, Type, Visibility,
    braced, parse::ParseStream, spanned::Spanned,
};

use crate::utils::byte_lit;

/// Status word pattern
#[derive(Debug, Clone)]
pub(crate) enum SwPattern {
    /// Match any value
    Any,
    /// Match an exact value
    Exact(u8),
    /// Match any value except a specific one
    Not(u8),
    /// Match a range of values
    Range(u8, u8),
    /// Match a path expression (constant)
    Path(TokenStream),
}

/// Status word annotation
#[derive(Debug, Clone)]
pub(crate) struct SwAnnotation {
    /// SW1 pattern
    pub sw1: SwPattern,
    /// SW2 pattern
    pub sw2: SwPattern,
    /// Status word reference if using a constant
    pub sw_ref: Option<TokenStream>,
}

/// Response variant
pub(crate) struct ResponseVariant {
    /// Variant name
    pub name: Ident,
    /// Status word pattern
    pub sw_pattern: SwAnnotation,
    /// Fields
    pub fields: Vec<Field>,
    /// Fields that capture SW1/SW2 values
    pub sw_fields: Vec<(String, bool)>, // (field_name, is_sw1)
    /// Documentation attributes
    pub doc_attrs: Vec<Attribute>,
}

/// Response definition parsed from the `response` block
pub(crate) struct ResponseDef {
    /// Variants in the response enum
    pub variants: Vec<ResponseVariant>,
    /// Custom payload parser
    pub payload_parser: Option<ExprClosure>,
    /// Methods
    pub methods: Vec<ItemFn>,
}

impl ResponseDef {
    /// Parse a response definition from a ParseStream
    pub(crate) fn parse<'a>(input: &'a ParseStream<'a>) -> syn::Result<Self> {
        let mut variants = Vec::new();
        let mut payload_parser = None;
        let mut methods = Vec::new();

        // Parse each field in the response block
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let key_str = key.to_string();

            match key_str.as_str() {
                "variants" => {
                    // Support both names for backward compatibility
                    // Parse enum response
                    let content;
                    braced!(content in input);

                    variants = Self::parse_variants(&&content)?;

                    // Try to parse comma if not at end
                    if !input.is_empty() {
                        let _ = input.parse::<Token![,]>();
                    }
                }
                "parse_payload" => {
                    // Parse custom payload parser
                    input.parse::<Token![=]>()?;
                    let parser: ExprClosure = input.parse()?;
                    payload_parser = Some(parser);

                    // Try to parse comma if not at end
                    if !input.is_empty() {
                        let _ = input.parse::<Token![,]>();
                    }
                }
                "methods" => {
                    // Parse methods
                    let methods_content;
                    braced!(methods_content in input);

                    while !methods_content.is_empty() {
                        let method: ItemFn = methods_content.parse()?;
                        methods.push(method);
                    }

                    // Try to parse comma if not at end
                    if !input.is_empty() {
                        let _ = input.parse::<Token![,]>();
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown response field: {}", key_str),
                    ));
                }
            }
        }

        // Ensure variants are specified
        if variants.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "Response must have 'variants' section with at least one variant",
            ));
        }

        Ok(Self {
            variants,
            payload_parser,
            methods,
        })
    }

    /// Parse variants for an enum response
    fn parse_variants<'a>(input: &'a ParseStream<'a>) -> syn::Result<Vec<ResponseVariant>> {
        let mut variants = Vec::new();

        while !input.is_empty() {
            // Get doc and other attributes
            let attrs = input.call(Attribute::parse_outer)?;

            // Extract doc attributes
            let doc_attrs = attrs
                .iter()
                .filter(|attr| attr.path().is_ident("doc"))
                .cloned()
                .collect::<Vec<_>>();

            // Look for sw attribute
            let mut sw_pattern = None;

            for attr in &attrs {
                if attr.path().is_ident("sw") {
                    // Parse SW pattern at the variant level
                    sw_pattern = Some(Self::parse_sw_attribute(attr)?);
                }
            }

            // Parse variant name
            let variant_name: Ident = input.parse()?;

            // Check if we have braces (for a struct-like variant)
            let has_fields = input.peek(syn::token::Brace);

            // Parse fields if present
            let mut fields = Vec::new();
            let mut sw_fields = Vec::new();

            if has_fields {
                let content;
                braced!(content in input);

                while !content.is_empty() {
                    // Parse field attributes
                    let field_attrs = content.call(Attribute::parse_outer)?;

                    // Parse the rest of the field
                    let vis = content.parse()?;
                    let name: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;
                    let ty: Type = content.parse()?;

                    // Create the field
                    let field = syn::Field {
                        attrs: field_attrs,
                        vis,
                        ident: Some(name.clone()),
                        colon_token: Some(Default::default()),
                        ty,
                        mutability: syn::FieldMutability::None,
                    };

                    fields.push(field);

                    // Check field name for sw1/sw2 convention
                    let name_str = name.to_string();
                    if name_str == "sw1" {
                        sw_fields.push((name_str, true));
                    } else if name_str == "sw2" {
                        sw_fields.push((name_str, false));
                    }

                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
            }

            // Ensure we have a valid SW pattern
            let sw_pattern = sw_pattern.ok_or_else(|| {
                syn::Error::new(variant_name.span(), "Missing #[sw] attribute for variant")
            })?;

            variants.push(ResponseVariant {
                name: variant_name,
                sw_pattern,
                fields,
                sw_fields,
                doc_attrs,
            });

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(variants)
    }

    /// Parse a status word attribute
    fn parse_sw_attribute(attr: &Attribute) -> syn::Result<SwAnnotation> {
        // Get the attribute contents from Meta
        let meta = &attr.meta;

        // For the new API (attributes above variant) we need to extract tokens differently
        let tokens = match meta {
            syn::Meta::List(list) => {
                // We have the raw tokens inside the list
                &list.tokens
            }
            _ => {
                return Err(syn::Error::new(
                    attr.span(),
                    "Expected #[sw(sw1, sw2)] format for SW attribute",
                ));
            }
        };

        // Try to parse as a tuple expression (sw1, sw2)
        if let Ok(tuple) = syn::parse2::<syn::ExprTuple>(tokens.clone()) {
            if tuple.elems.len() != 2 {
                return Err(syn::Error::new(
                    attr.span(),
                    "SW attribute must have two arguments (sw1, sw2)",
                ));
            }

            // Extract the SW1 and SW2 patterns
            let sw1_expr = &tuple.elems[0];
            let sw2_expr = &tuple.elems[1];

            let sw1 = Self::parse_sw_component(sw1_expr)?;
            let sw2 = Self::parse_sw_component(sw2_expr)?;

            return Ok(SwAnnotation {
                sw1,
                sw2,
                sw_ref: None,
            });
        }

        // If not a tuple, try to parse as a single path expression (like status::SUCCESS)
        if let Ok(expr) = syn::parse2::<syn::Expr>(tokens.clone()) {
            match &expr {
                Expr::Path(path) => {
                    // This is a reference to a StatusWord constant
                    let sw_ref = quote! { #path };
                    return Ok(SwAnnotation {
                        sw1: SwPattern::Path(quote! { #path.sw1() }),
                        sw2: SwPattern::Path(quote! { #path.sw2() }),
                        sw_ref: Some(sw_ref),
                    });
                }
                _ => {
                    // Special handling for underscore
                    if let Some(lit) = extract_token_str(tokens) {
                        if lit == "_" {
                            return Ok(SwAnnotation {
                                sw1: SwPattern::Any,
                                sw2: SwPattern::Any,
                                sw_ref: None,
                            });
                        }
                    }

                    return Err(syn::Error::new(
                        expr.span(),
                        "Expected a path to StatusWord constant or a tuple (sw1, sw2)",
                    ));
                }
            }
        }

        // Last resort: Try to manually parse as comma-separated values
        if let Some(lit) = extract_token_str(tokens) {
            let parts: Vec<&str> = lit.split(',').map(|s| s.trim()).collect();

            if parts.len() == 2 {
                let sw1_str = parts[0];
                let sw2_str = parts[1];

                // Parse SW1
                let sw1 = if sw1_str == "_" {
                    SwPattern::Any
                } else if let Some(val_str) = sw1_str.strip_prefix("!") {
                    if let Ok(val) = u8::from_str_radix(val_str.trim_start_matches("0x"), 16) {
                        SwPattern::Not(val)
                    } else {
                        return Err(syn::Error::new(
                            attr.span(),
                            format!("Invalid SW1 value: {}", sw1_str),
                        ));
                    }
                } else if sw1_str.starts_with("0x") {
                    if let Ok(val) = u8::from_str_radix(sw1_str.trim_start_matches("0x"), 16) {
                        SwPattern::Exact(val)
                    } else {
                        return Err(syn::Error::new(
                            attr.span(),
                            format!("Invalid SW1 value: {}", sw1_str),
                        ));
                    }
                } else {
                    // Try as a hex number without 0x prefix
                    if let Ok(val) = u8::from_str_radix(sw1_str, 16) {
                        SwPattern::Exact(val)
                    } else {
                        return Err(syn::Error::new(
                            attr.span(),
                            format!("Invalid SW1 value: {}", sw1_str),
                        ));
                    }
                };

                // Parse SW2
                let sw2 = if sw2_str == "_" {
                    SwPattern::Any
                } else if let Some(val_str) = sw2_str.strip_prefix("!") {
                    if let Ok(val) = u8::from_str_radix(val_str.trim_start_matches("0x"), 16) {
                        SwPattern::Not(val)
                    } else {
                        return Err(syn::Error::new(
                            attr.span(),
                            format!("Invalid SW2 value: {}", sw2_str),
                        ));
                    }
                } else if sw2_str.starts_with("0x") {
                    if let Ok(val) = u8::from_str_radix(sw2_str.trim_start_matches("0x"), 16) {
                        SwPattern::Exact(val)
                    } else {
                        return Err(syn::Error::new(
                            attr.span(),
                            format!("Invalid SW2 value: {}", sw2_str),
                        ));
                    }
                } else {
                    // Try as a hex number without 0x prefix
                    if let Ok(val) = u8::from_str_radix(sw2_str, 16) {
                        SwPattern::Exact(val)
                    } else {
                        return Err(syn::Error::new(
                            attr.span(),
                            format!("Invalid SW2 value: {}", sw2_str),
                        ));
                    }
                };

                return Ok(SwAnnotation {
                    sw1,
                    sw2,
                    sw_ref: None,
                });
            }
        }

        // If we get here, we couldn't parse the attribute
        Err(syn::Error::new(
            attr.span(),
            "Expected either (sw1, sw2) format or a StatusWord constant reference",
        ))
    }

    /// Parse a status word component (SW1 or SW2)
    fn parse_sw_component(expr: &Expr) -> syn::Result<SwPattern> {
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Int(lit_int),
                ..
            }) => {
                // Numerical literal
                Ok(SwPattern::Exact(lit_int.base10_parse()?))
            }
            Expr::Path(path) if path.path.is_ident("_") => {
                // Wildcard _ (as a path)
                Ok(SwPattern::Any)
            }
            Expr::Unary(unary) => {
                // Negation !0x00
                if let syn::UnOp::Not(_) = unary.op {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Int(lit_int),
                        ..
                    }) = &*unary.expr
                    {
                        Ok(SwPattern::Not(lit_int.base10_parse()?))
                    } else {
                        Err(syn::Error::new(
                            unary.expr.span(),
                            "Expected integer literal after !",
                        ))
                    }
                } else {
                    Err(syn::Error::new(
                        unary.span(),
                        "Expected ! operator for negation pattern",
                    ))
                }
            }
            Expr::Path(_) => {
                // Variable/constant reference
                Ok(SwPattern::Path(quote! { #expr }))
            }
            _ => Err(syn::Error::new(
                expr.span(),
                "Expected integer literal, _, !value, or constant reference",
            )),
        }
    }
}

/// Extract a token string for fallback parsing
fn extract_token_str(tokens: &TokenStream) -> Option<String> {
    let s = tokens.to_string();
    let s = s.trim();

    // Remove outer parentheses if present
    if s.starts_with('(') && s.ends_with(')') {
        Some(s[1..s.len() - 1].to_string())
    } else {
        Some(s.to_string())
    }
}

/// Expand a response definition into an enum
pub(crate) fn expand_response(
    response: &ResponseDef,
    vis: &Visibility,
    response_name: &Ident,
    _command_name: &Ident,
) -> Result<TokenStream, syn::Error> {
    // Create variants for the enum
    let enum_variants = response.variants.iter().map(|v| {
        let name = &v.name;
        let fields = &v.fields;
        let doc_attrs = &v.doc_attrs;

        if fields.is_empty() {
            quote! {
                #(#doc_attrs)*
                #name
            }
        } else {
            quote! {
                #(#doc_attrs)*
                #name { #(#fields,)* }
            }
        }
    });

    // Generate match arms for from_bytes
    let match_arms = response.variants.iter().map(|v| {
        let name = &v.name;
        let sw_pattern = &v.sw_pattern;

        // Generate the match expression
        let match_expr = &sw_pattern.sw_ref.as_ref().map_or_else(
            || {
                // Otherwise, construct the match pattern from individual SW1/SW2 patterns
                // Generate SW1 pattern
                let sw1_pattern = match &sw_pattern.sw1 {
                    SwPattern::Any => quote! { _ },
                    SwPattern::Exact(val) => {
                        let lit = byte_lit(*val);
                        quote! { #lit }
                    }
                    SwPattern::Not(val) => {
                        let lit = byte_lit(*val);
                        quote! { sw1 if sw1 != #lit }
                    }
                    SwPattern::Range(start, end) => {
                        let start_lit = byte_lit(*start);
                        let end_lit = byte_lit(*end);
                        quote! { sw1 if sw1 >= #start_lit && sw1 <= #end_lit }
                    }
                    SwPattern::Path(path) => {
                        quote! { sw1 if sw1 == #path }
                    }
                };

                // Generate SW2 pattern
                let sw2_pattern = match &sw_pattern.sw2 {
                    SwPattern::Any => quote! { _ },
                    SwPattern::Exact(val) => {
                        let lit = byte_lit(*val);
                        quote! { #lit }
                    }
                    SwPattern::Not(val) => {
                        let lit = byte_lit(*val);
                        quote! { sw2 if sw2 != #lit }
                    }
                    SwPattern::Range(start, end) => {
                        let start_lit = byte_lit(*start);
                        let end_lit = byte_lit(*end);
                        quote! { sw2 if sw2 >= #start_lit && sw2 <= #end_lit }
                    }
                    SwPattern::Path(path) => {
                        quote! { sw2 if sw2 == #path }
                    }
                };

                quote! {
                    (#sw1_pattern, #sw2_pattern)
                }
            },
            |sw_ref| {
                quote! {
                    (sw1, sw2) if apdu_core::StatusWord::new(sw1, sw2) == #sw_ref
                }
            },
        );

        // Initialize fields
        let field_inits = v.fields.iter().map(|f| {
            let name = &f.ident.as_ref().unwrap();
            let name_str = name.to_string();

            // Check if this field should capture SW1 or SW2
            let is_sw1_field = v
                .sw_fields
                .iter()
                .any(|(field_name, is_sw1)| field_name == &name_str && *is_sw1);
            let is_sw2_field = v
                .sw_fields
                .iter()
                .any(|(field_name, is_sw1)| field_name == &name_str && !*is_sw1);

            if is_sw1_field {
                // Capture SW1
                quote! { #name: sw1 }
            } else if is_sw2_field {
                // Capture SW2
                quote! { #name: sw2 }
            } else {
                // Regular field - initialize to default value
                quote! { #name: Default::default() }
            }
        });

        if v.fields.is_empty() {
            // Unit variant
            quote! {
                #match_expr => Self::#name
            }
        } else {
            // Struct-like variant
            quote! {
                #match_expr => {
                    Self::#name { #(#field_inits,)* }
                }
            }
        }
    });

    // Generate payload parsing code if a parser was provided
    let payload_parsing = &response.payload_parser.as_ref().map_or_else(
        || {
            quote! {
                // No custom parser, just use default values
            }
        },
        |parser| {
            quote! {
                // Use the custom payload parser if provided
                let status_word = apdu_core::StatusWord::new(sw1, sw2);
                // Apply the custom parser
                (#parser)(payload, status_word, &mut response)?;
            }
        },
    );

    // Include all the user-defined methods
    let user_methods = &response.methods;

    let tokens = quote! {
        /// APDU response
        #[derive(Debug, Clone)]
        #vis enum #response_name {
            #(#enum_variants,)*
        }

        impl #response_name {
            /// Parse response from raw bytes
            pub fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, apdu_core::Error> {
                if bytes.len() < 2 {
                    return Err(apdu_core::Error::Response(
                        apdu_core::response::error::ResponseError::Incomplete
                    ));
                }

                let sw1 = bytes[bytes.len() - 2];
                let sw2 = bytes[bytes.len() - 1];

                let payload = if bytes.len() > 2 {
                    &bytes[..bytes.len() - 2]
                } else {
                    &[]
                };

                // Create the initial response variant based on status
                let mut response = match (sw1, sw2) {
                    #(#match_arms,)*
                    _ => return Err(apdu_core::Error::status(sw1, sw2)),
                };

                // Apply custom payload parsing if payload is present
                if !payload.is_empty() {
                    #payload_parsing
                }

                Ok(response)
            }

            // Include all user-defined methods
            #(#user_methods)*
        }

        impl TryFrom<bytes::Bytes> for #response_name {
            type Error = apdu_core::Error;

            fn try_from(bytes: bytes::Bytes) -> core::result::Result<Self, Self::Error> {
                Self::from_bytes(&bytes)
            }
        }
    };

    Ok(tokens)
}
