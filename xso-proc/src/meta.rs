// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! # Parse Rust attributes
//!
//! This module is concerned with parsing attributes from the Rust "meta"
//! annotations on structs, enums, enum variants and fields.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, *};

use rxml_validation::NcName;

/// Value for the `#[xml(namespace = ..)]` attribute.
#[derive(Debug)]
pub(crate) enum NamespaceRef {
    /// The XML namespace is specified as a string literal.
    LitStr(LitStr),

    /// The XML namespace is specified as a path.
    Path(Path),
}

impl syn::parse::Parse for NamespaceRef {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        if input.peek(syn::LitStr) {
            Ok(Self::LitStr(input.parse()?))
        } else {
            Ok(Self::Path(input.parse()?))
        }
    }
}

impl quote::ToTokens for NamespaceRef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::LitStr(ref lit) => lit.to_tokens(tokens),
            Self::Path(ref path) => path.to_tokens(tokens),
        }
    }
}

/// Value for the `#[xml(name = .. )]` attribute.
#[derive(Debug)]
pub(crate) enum NameRef {
    /// The XML name is specified as a string literal.
    Literal {
        /// The validated XML name.
        value: NcName,

        /// The span of the original [`syn::LitStr`].
        span: Span,
    },

    /// The XML name is specified as a path.
    Path(Path),
}

impl syn::parse::Parse for NameRef {
    fn parse(input: syn::parse::ParseStream<'_>) -> Result<Self> {
        if input.peek(syn::LitStr) {
            let s: LitStr = input.parse()?;
            let span = s.span();
            match NcName::try_from(s.value()) {
                Ok(value) => Ok(Self::Literal { value, span }),
                Err(e) => Err(Error::new(
                    span,
                    format!("not a valid XML element name: {}", e),
                )),
            }
        } else {
            let p: Path = input.parse()?;
            Ok(Self::Path(p))
        }
    }
}

impl quote::ToTokens for NameRef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Literal { ref value, span } => {
                let span = *span;
                let value = value.as_str();
                let value = quote_spanned! { span=> #value };
                // SAFETY: self.0 is a known-good NcName, so converting it to an
                // NcNameStr is known to be safe.
                // NOTE: we cannot use `quote_spanned! { self.span=> }` for the unsafe
                // block as that would then in fact trip a `#[deny(unsafe_code)]` lint
                // at the use site of the macro.
                tokens.extend(quote! {
                    unsafe { ::xso::exports::rxml::NcNameStr::from_str_unchecked(#value) }
                })
            }
            Self::Path(ref path) => path.to_tokens(tokens),
        }
    }
}

/// Contents of an `#[xml(..)]` attribute on a struct, enum variant, or enum.
#[derive(Debug)]
pub(crate) struct XmlCompoundMeta {
    /// The span of the `#[xml(..)]` meta from which this was parsed.
    ///
    /// This is useful for error messages.
    pub(crate) span: Span,

    /// The value assigned to `namespace` inside `#[xml(..)]`, if any.
    pub(crate) namespace: Option<NamespaceRef>,

    /// The value assigned to `name` inside `#[xml(..)]`, if any.
    pub(crate) name: Option<NameRef>,
}

impl XmlCompoundMeta {
    /// Parse the meta values from a `#[xml(..)]` attribute.
    ///
    /// Undefined options or options with incompatible values are rejected
    /// with an appropriate compile-time error.
    fn parse_from_attribute(attr: &Attribute) -> Result<Self> {
        let mut namespace = None;
        let mut name = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if name.is_some() {
                    return Err(Error::new_spanned(meta.path, "duplicate `name` key"));
                }
                name = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("namespace") {
                if namespace.is_some() {
                    return Err(Error::new_spanned(meta.path, "duplicate `namespace` key"));
                }
                namespace = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(Error::new_spanned(meta.path, "unsupported key"))
            }
        })?;

        Ok(Self {
            span: attr.span(),
            namespace,
            name,
        })
    }

    /// Search through `attrs` for a single `#[xml(..)]` attribute and parse
    /// it.
    ///
    /// Undefined options or options with incompatible values are rejected
    /// with an appropriate compile-time error.
    ///
    /// If more than one `#[xml(..)]` attribute is found, an error is
    /// emitted.
    ///
    /// If no `#[xml(..)]` attribute is found, `None` is returned.
    pub(crate) fn try_parse_from_attributes(attrs: &[Attribute]) -> Result<Option<Self>> {
        let mut result = None;
        for attr in attrs {
            if !attr.path().is_ident("xml") {
                continue;
            }
            if result.is_some() {
                return Err(syn::Error::new_spanned(
                    attr.path(),
                    "only one #[xml(..)] per struct or enum variant allowed",
                ));
            }
            result = Some(Self::parse_from_attribute(attr)?);
        }
        Ok(result)
    }

    /// Search through `attrs` for a single `#[xml(..)]` attribute and parse
    /// it.
    ///
    /// Undefined options or options with incompatible values are rejected
    /// with an appropriate compile-time error.
    ///
    /// If more than one or no `#[xml(..)]` attribute is found, an error is
    /// emitted.
    pub(crate) fn parse_from_attributes(attrs: &[Attribute]) -> Result<Self> {
        match Self::try_parse_from_attributes(attrs)? {
            Some(v) => Ok(v),
            None => Err(syn::Error::new(
                Span::call_site(),
                "#[xml(..)] attribute required on struct or enum variant",
            )),
        }
    }
}
