// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! # Parse Rust attributes
//!
//! This module is concerned with parsing attributes from the Rust "meta"
//! annotations on structs, enums, enum variants and fields.

use proc_macro2::Span;
use syn::{spanned::Spanned, *};

/// Type alias for a `#[xml(namespace = ..)]` attribute.
///
/// This may, in the future, be replaced by an enum supporting multiple
/// ways to specify a namespace.
pub(crate) type NamespaceRef = Path;

/// Type alias for a `#[xml(name = ..)]` attribute.
///
/// This may, in the future, be replaced by an enum supporting both `Path` and
/// `LitStr`.
pub(crate) type NameRef = LitStr;

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
