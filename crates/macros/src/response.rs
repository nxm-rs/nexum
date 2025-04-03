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
#[derive(Clone)]
pub(crate) struct ResponseVariant {
    /// Variant name
    pub name: Ident,
    /// Status word pattern
    pub sw_pattern: SwAnnotation,
    /// Fields
    pub fields: Vec<Field>,
    /// Fields that capture SW1/SW2 values
    pub sw_fields: Vec<(String, bool)>, // (field_name, is_sw1)
    /// Field to receive payload data (if any)
    pub payload_field: Option<String>,
    /// Documentation attributes
    pub doc_attrs: Vec<Attribute>,
    /// Error message for error variants
    pub error_attr: Option<Attribute>,
    /// Whether this is an error variant
    pub is_error: bool,
}

/// Response definition parsed from the `response` block
pub(crate) struct ResponseDef {
    /// Ok variants in the response enum
    pub ok_variants: Vec<ResponseVariant>,
    /// Error variants in the response enum
    pub error_variants: Vec<ResponseVariant>,
    /// Custom response parser
    pub custom_parser: Option<ExprClosure>,
    /// Methods
    pub methods: Vec<ItemFn>,
}

impl ResponseDef {
    /// Parse a response definition from a ParseStream
    pub(crate) fn parse<'a>(input: &'a ParseStream<'a>) -> syn::Result<Self> {
        let mut ok_variants = Vec::new();
        let mut error_variants = Vec::new();
        let mut custom_parser = None;
        let mut methods = Vec::new();

        // Parse each field in the response block
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let key_str = key.to_string();

            match key_str.as_str() {
                "ok" => {
                    // Parse ok variants (success responses)
                    let content;
                    braced!(content in input);

                    let parsed_variants = Self::parse_variants(&&content, false)?;
                    ok_variants.extend(parsed_variants);

                    // Try to parse comma if not at end
                    if !input.is_empty() {
                        let _ = input.parse::<Token![,]>();
                    }
                }
                "errors" => {
                    // Parse error variants
                    let content;
                    braced!(content in input);

                    let parsed_variants = Self::parse_variants(&&content, true)?;
                    error_variants.extend(parsed_variants);

                    // Try to parse comma if not at end
                    if !input.is_empty() {
                        let _ = input.parse::<Token![,]>();
                    }
                }
                "variants" => {
                    // Legacy variants section - provide a helpful error
                    return Err(syn::Error::new(
                        key.span(),
                        "The 'variants' section is no longer supported. Use 'ok' and 'errors' sections instead.",
                    ));
                }
                "custom_parse" => {
                    // Parse custom parser
                    input.parse::<Token![=]>()?;
                    let parser: ExprClosure = input.parse()?;
                    custom_parser = Some(parser);

                    // Try to parse comma if not at end
                    if !input.is_empty() {
                        let _ = input.parse::<Token![,]>();
                    }
                }
                "parse_payload" => {
                    // Legacy parse_payload - generate a warning but ignore
                    return Err(syn::Error::new(
                        key.span(),
                        "parse_payload is deprecated. Use custom_parse instead.",
                    ));
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

        // Ensure at least one of ok or errors is specified
        if ok_variants.is_empty() && error_variants.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "Response must have at least one variant in either 'ok' or 'errors' section",
            ));
        }

        Ok(Self {
            ok_variants,
            error_variants,
            custom_parser,
            methods,
        })
    }

    /// Parse variants for an enum response
    fn parse_variants<'a>(
        input: &'a ParseStream<'a>,
        is_error: bool,
    ) -> syn::Result<Vec<ResponseVariant>> {
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

            // Look for sw attribute and error attribute
            let mut sw_pattern = None;
            let mut payload_field = None;
            let mut error_message = None;

            for attr in &attrs {
                if attr.path().is_ident("sw") {
                    // Parse SW pattern at the variant level
                    sw_pattern = Some(Self::parse_sw_attribute(attr)?);
                } else if attr.path().is_ident("payload") {
                    // Parse payload field attribute
                    payload_field = Self::parse_payload_attribute(attr)?;
                } else if attr.path().is_ident("error") && is_error {
                    // Parse error message attribute for error variants
                    error_message = Self::parse_error_attribute(attr)?;
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
                        sw_fields.push((name_str.clone(), true));
                    } else if name_str == "sw2" {
                        sw_fields.push((name_str.clone(), false));
                    }

                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }

                // Validate payload field name if specified
                if let Some(ref field_name) = payload_field {
                    if !fields
                        .iter()
                        .any(|f| f.ident.as_ref().is_some_and(|ident| ident == field_name))
                    {
                        return Err(syn::Error::new(
                            variant_name.span(),
                            format!(
                                "Payload field '{}' not found in variant '{}'",
                                field_name, variant_name
                            ),
                        ));
                    }
                }
            }

            // Ensure we have a valid SW pattern
            let sw_pattern = sw_pattern.ok_or_else(|| {
                syn::Error::new(variant_name.span(), "Missing #[sw] attribute for variant")
            })?;

            // For error variants, ensure we have an error message unless it's a struct variant
            if is_error && error_message.is_none() && fields.is_empty() {
                return Err(syn::Error::new(
                    variant_name.span(),
                    "Error variants must have an #[error(\"message\")] attribute",
                ));
            }

            variants.push(ResponseVariant {
                name: variant_name,
                sw_pattern,
                fields,
                sw_fields,
                payload_field,
                doc_attrs,
                error_attr: error_message,
                is_error,
            });

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(variants)
    }

    /// Parse a payload field attribute
    fn parse_payload_attribute(attr: &Attribute) -> syn::Result<Option<String>> {
        // Check for #[payload(field = "name")]
        let meta = &attr.meta;

        match meta {
            syn::Meta::List(list) => {
                let nested_meta = syn::parse2::<syn::Meta>(quote::quote! { #list })?;

                if let syn::Meta::NameValue(nv) = nested_meta {
                    if nv.path.is_ident("field") {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit_str),
                            ..
                        }) = &nv.value
                        {
                            return Ok(Some(lit_str.value()));
                        }
                    }
                }

                // Try parsing in a more manual way
                let content = list.tokens.to_string();
                let field_str = "field = \"";
                if let Some(start) = content.find(field_str) {
                    let start = start + field_str.len();
                    if let Some(end) = content[start..].find('\"') {
                        let field_name = content[start..(start + end)].to_string();
                        return Ok(Some(field_name));
                    }
                }

                Err(syn::Error::new(
                    list.span(),
                    "Expected #[payload(field = \"field_name\")] format",
                ))
            }
            _ => Err(syn::Error::new(
                attr.span(),
                "Expected #[payload(field = \"field_name\")] format",
            )),
        }
    }

    /// Parse an error message attribute
    fn parse_error_attribute(attr: &Attribute) -> syn::Result<Option<Attribute>> {
        // Just validate the attribute is formatted correctly
        let meta = &attr.meta;
        match meta {
            syn::Meta::List(list) => {
                if syn::parse2::<syn::LitStr>(list.tokens.clone()).is_ok() {
                    // It's valid, return the original attribute
                    Ok(Some(attr.clone()))
                } else {
                    Err(syn::Error::new(
                        list.span(),
                        "Expected #[error(\"message\")] format",
                    ))
                }
            }
            _ => Err(syn::Error::new(
                attr.span(),
                "Expected #[error(\"message\")] format",
            )),
        }
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
                    if let Some(lit) = Self::extract_token_str(tokens) {
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
        if let Some(lit) = Self::extract_token_str(tokens) {
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
}

/// Expand a response definition into appropriate enums and implementations
pub(crate) fn expand_response(
    response: &ResponseDef,
    vis: &Visibility,
    response_name: &Ident,
    _command_name: &Ident,
) -> Result<(TokenStream, Ident, Ident, Ident), syn::Error> {
    // Generate the base name for the Ok and Error enums
    let struct_base = response_name
        .to_string()
        .trim_end_matches("Response")
        .to_string();
    let ok_enum_name = Ident::new(&format!("{}Ok", struct_base), Span::call_site());
    let error_enum_name = Ident::new(&format!("{}Error", struct_base), Span::call_site());
    let result_type_name = Ident::new(&format!("{}Result", struct_base), Span::call_site());

    // Collect all variants for the main response enum
    let mut all_variants = Vec::new();
    all_variants.extend(response.ok_variants.clone());
    all_variants.extend(response.error_variants.clone());

    // Generate variants for the main response enum
    let enum_variants = all_variants.iter().map(|v| {
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

    // Generate variants for the Ok enum
    let ok_variants = response.ok_variants.iter().map(|v| {
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

    // Generate variants for the Error enum
    let error_variants = response.error_variants.iter().map(|v| {
        let name = &v.name;
        let fields = &v.fields;
        let doc_attrs = &v.doc_attrs;

        // Add #[error("message")] attribute for thiserror
        let error_attrs = if let Some(ref error_attr) = v.error_attr {
            quote! { #error_attr }
        } else {
            quote! {}
        };

        if fields.is_empty() {
            quote! {
                #(#doc_attrs)*
                #error_attrs
                #name
            }
        } else {
            quote! {
                #(#doc_attrs)*
                #error_attrs
                #name { #(#fields,)* }
            }
        }
    });

    // Generate match arms for from_bytes
    let match_arms = all_variants.iter().map(|v| {
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
                    (sw1, sw2) if nexum_apdu_core::StatusWord::new(sw1, sw2) == #sw_ref
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

            // Check if this field is the payload field
            let is_payload_field = v.payload_field.as_ref() == Some(&name_str);

            if is_sw1_field {
                // Capture SW1
                quote! { #name: sw1 }
            } else if is_sw2_field {
                // Capture SW2
                quote! { #name: sw2 }
            } else if is_payload_field {
                // Handle payload field based on its type
                let ty = &f.ty;
                let ty_str = quote! { #ty }.to_string();

                if ty_str.contains("Vec < u8 >") || ty_str.contains("bytes :: Bytes") {
                    // For Vec<u8> or bytes::Bytes, copy the payload directly
                    quote! { #name: payload.to_vec() }
                } else if ty_str.contains("Option < Vec < u8 > >") || ty_str.contains("Option < bytes :: Bytes >") {
                    // For Option<Vec<u8>> or Option<bytes::Bytes>, wrap in Some
                    quote! { #name: if !payload.is_empty() { Some(payload.to_vec()) } else { None } }
                } else if ty_str.contains("String") {
                    // For String, try to convert from UTF-8
                    quote! { #name: core::str::from_utf8(payload).unwrap_or_default().to_string() }
                } else {
                    // Default to basic copy for other types
                    quote! { #name: Default::default() }
                }
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

    // Generate to_result() match arms to convert from Response to Result<Ok, Error>
    let to_result_arms = all_variants.iter().map(|v| {
        let name = &v.name;
        let is_error = v.is_error;

        if v.fields.is_empty() {
            // Handle unit variants
            if is_error {
                quote! {
                    #response_name::#name => Err(#error_enum_name::#name)
                }
            } else {
                quote! {
                    #response_name::#name => Ok(#ok_enum_name::#name)
                }
            }
        } else {
            // Handle struct variants
            let field_names: Vec<_> = v.fields.iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();

            if is_error {
                quote! {
                    #response_name::#name { #(#field_names,)* } => Err(#error_enum_name::#name { #(#field_names,)* })
                }
            } else {
                quote! {
                    #response_name::#name { #(#field_names,)* } => Ok(#ok_enum_name::#name { #(#field_names,)* })
                }
            }
        }
    });

    // Custom parser code
    let parsing_logic = &response.custom_parser.as_ref().map_or_else(|| quote! {
        let response = match (sw1, sw2) {
            #(#match_arms,)*
            _ => return Err(nexum_apdu_core::response::error::ResponseError::status(sw1, sw2)),
        };

        Ok(response)
    }, |custom_parser| quote! {
        let sw = nexum_apdu_core::StatusWord::new(sw1, sw2);
        (#custom_parser)(payload, sw)
    });

    // Include all the user-defined methods
    let user_methods = &response.methods;

    // Generate the code
    let tokens = quote! {
        /// APDU response
        #[derive(Debug, Clone)]
        #vis enum #response_name {
            #(#enum_variants,)*
        }

        /// Successful response variants
        #[derive(Debug, Clone)]
        #vis enum #ok_enum_name {
            #(#ok_variants,)*
        }

        /// Error response variants
        #[derive(Debug, Clone, thiserror::Error)]
        #vis enum #error_enum_name {
            #(#error_variants,)*
        }

        /// Type alias for Result with the appropriate success and error types
        #vis type #result_type_name = Result<#ok_enum_name, #error_enum_name>;

        impl #response_name {
            /// Parse response from raw bytes
            pub fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, nexum_apdu_core::response::error::ResponseError> {
                if bytes.len() < 2 {
                    return Err(nexum_apdu_core::response::error::ResponseError::Incomplete);
                }

                let sw1 = bytes[bytes.len() - 2];
                let sw2 = bytes[bytes.len() - 1];

                let payload = if bytes.len() > 2 {
                    &bytes[..bytes.len() - 2]
                } else {
                    &[]
                };

                // Process the response using either custom or default parser
                #parsing_logic
            }

            /// Convert the response to a Result
            pub fn to_result(self) -> #result_type_name {
                match self {
                    #(#to_result_arms,)*
                }
            }

            // Include all user-defined methods
            #(#user_methods)*
        }

        impl TryFrom<bytes::Bytes> for #response_name {
            type Error = nexum_apdu_core::response::error::ResponseError;

            fn try_from(bytes: bytes::Bytes) -> core::result::Result<Self, Self::Error> {
                Self::from_bytes(&bytes)
            }
        }

        impl From<#response_name> for #result_type_name {
            fn from(response: #response_name) -> Self {
                response.to_result()
            }
        }

        impl From<#error_enum_name> for nexum_apdu_core::Error {
            fn from(err: #error_enum_name) -> Self {
                nexum_apdu_core::Error::Response(
                    nexum_apdu_core::response::error::ResponseError::Message(err.to_string())
                )
            }
        }
    };

    Ok((tokens, ok_enum_name, error_enum_name, result_type_name))
}
