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
use syn::{meta::ParseNestedMeta, spanned::Spanned, *};

use rxml_validation::NcName;

/// XML core namespace URI (for the `xml:` prefix)
pub const XMLNS_XML: &'static str = "http://www.w3.org/XML/1998/namespace";
/// XML namespace URI (for the `xmlns:` prefix)
pub const XMLNS_XMLNS: &'static str = "http://www.w3.org/2000/xmlns/";

/// Value for the `#[xml(namespace = ..)]` attribute.
#[derive(Debug)]
pub(crate) enum NamespaceRef {
    /// The XML namespace is specified as a string literal.
    LitStr(LitStr),

    /// The XML namespace is specified as a path.
    Path(Path),
}

impl NamespaceRef {
    fn fudge(value: &str, span: Span) -> Self {
        Self::LitStr(LitStr::new(value, span))
    }
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
                Err(e) => Err(Error::new(span, format!("not a valid XML name: {}", e))),
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

/// Represents a boolean flag from a `#[xml(..)]` attribute meta.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Flag {
    /// The flag is not set.
    Absent,

    /// The flag was set.
    Present(
        /// The span of the syntax element which enabled the flag.
        ///
        /// This is used to generate useful error messages by pointing at the
        /// specific place the flag was activated.
        #[allow(dead_code)]
        Span,
    ),
}

impl Flag {
    /// Return true if the flag is set, false otherwise.
    pub(crate) fn is_set(&self) -> bool {
        match self {
            Self::Absent => false,
            Self::Present(_) => true,
        }
    }
}

impl<T: Spanned> From<T> for Flag {
    fn from(other: T) -> Flag {
        Flag::Present(other.span())
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

    /// The debug flag.
    pub(crate) debug: Flag,
}

impl XmlCompoundMeta {
    /// Parse the meta values from a `#[xml(..)]` attribute.
    ///
    /// Undefined options or options with incompatible values are rejected
    /// with an appropriate compile-time error.
    fn parse_from_attribute(attr: &Attribute) -> Result<Self> {
        let mut namespace = None;
        let mut name = None;
        let mut debug = Flag::Absent;

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
            } else if meta.path.is_ident("debug") {
                if debug.is_set() {
                    return Err(Error::new_spanned(meta.path, "duplicate `debug` key"));
                }
                debug = (&meta.path).into();
                Ok(())
            } else {
                Err(Error::new_spanned(meta.path, "unsupported key"))
            }
        })?;

        Ok(Self {
            span: attr.span(),
            namespace,
            name,
            debug,
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

/// Parse an XML name while resolving built-in namespace prefixes.
fn parse_prefixed_name(
    value: syn::parse::ParseStream<'_>,
) -> Result<(Option<NamespaceRef>, NameRef)> {
    if !value.peek(LitStr) {
        // if we don't have a string literal next, we delegate to the default
        // `NameRef` parser.
        return Ok((None, value.parse()?));
    }

    let name: LitStr = value.parse()?;
    let name_span = name.span();
    let (prefix, name) = match name
        .value()
        .try_into()
        .and_then(|name: rxml_validation::Name| name.split_name())
    {
        Ok(v) => v,
        Err(e) => {
            return Err(Error::new(
                name_span,
                format!("not a valid XML name: {}", e),
            ))
        }
    };
    let name = NameRef::Literal {
        value: name,
        span: name_span,
    };
    if let Some(prefix) = prefix {
        let namespace_uri = match prefix.as_str() {
            "xml" => XMLNS_XML,
            "xmlns" => XMLNS_XMLNS,
            other => return Err(Error::new(
                name_span,
                format!("prefix `{}` is not a built-in prefix and cannot be used. specify the desired namespace using the `namespace` key instead.", other)
            )),
        };
        Ok((Some(NamespaceRef::fudge(namespace_uri, name_span)), name))
    } else {
        Ok((None, name))
    }
}

/// Contents of an `#[xml(..)]` attribute on a struct or enum variant member.
#[derive(Debug)]
pub(crate) enum XmlFieldMeta {
    /// `#[xml(attribute)]`, `#[xml(attribute = ..)]` or `#[xml(attribute(..))]`
    Attribute {
        /// The span of the `#[xml(attribute)]` meta from which this was parsed.
        ///
        /// This is useful for error messages.
        span: Span,

        /// The XML namespace supplied.
        namespace: Option<NamespaceRef>,

        /// The XML name supplied.
        name: Option<NameRef>,

        /// The `default` flag.
        default_: Flag,
    },

    /// `#[xml(text)]`
    Text {
        /// The path to the optional codec type.
        codec: Option<Type>,
    },
}

impl XmlFieldMeta {
    /// Parse a `#[xml(attribute(..))]` meta.
    ///
    /// That meta can have three distinct syntax styles:
    /// - argument-less: `#[xml(attribute)]`
    /// - shorthand: `#[xml(attribute = ..)]`
    /// - full: `#[xml(attribute(..))]`
    fn attribute_from_meta(meta: ParseNestedMeta<'_>) -> Result<Self> {
        if meta.input.peek(Token![=]) {
            // shorthand syntax
            let (namespace, name) = parse_prefixed_name(meta.value()?)?;
            Ok(Self::Attribute {
                span: meta.path.span(),
                name: Some(name),
                namespace,
                default_: Flag::Absent,
            })
        } else if meta.input.peek(syn::token::Paren) {
            // full syntax
            let mut name: Option<NameRef> = None;
            let mut namespace: Option<NamespaceRef> = None;
            let mut default_ = Flag::Absent;
            meta.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    if name.is_some() {
                        return Err(Error::new_spanned(meta.path, "duplicate `name` key"));
                    }
                    let value = meta.value()?;
                    let name_span = value.span();
                    let (new_namespace, new_name) = parse_prefixed_name(value)?;
                    if let Some(new_namespace) = new_namespace {
                        if namespace.is_some() {
                            return Err(Error::new(
                                name_span,
                                "cannot combine `namespace` key with prefixed `name`",
                            ));
                        }
                        namespace = Some(new_namespace);
                    }
                    name = Some(new_name);
                    Ok(())
                } else if meta.path.is_ident("namespace") {
                    if namespace.is_some() {
                        return Err(Error::new_spanned(
                            meta.path,
                            "duplicate `namespace` key or `name` key has prefix",
                        ));
                    }
                    namespace = Some(meta.value()?.parse()?);
                    Ok(())
                } else if meta.path.is_ident("default") {
                    if default_.is_set() {
                        return Err(Error::new_spanned(meta.path, "duplicate `default` key"));
                    }
                    default_ = (&meta.path).into();
                    Ok(())
                } else {
                    Err(Error::new_spanned(meta.path, "unsupported key"))
                }
            })?;
            Ok(Self::Attribute {
                span: meta.path.span(),
                name,
                namespace,
                default_,
            })
        } else {
            // argument-less syntax
            Ok(Self::Attribute {
                span: meta.path.span(),
                name: None,
                namespace: None,
                default_: Flag::Absent,
            })
        }
    }

    /// Parse a `#[xml(text)]` meta.
    fn text_from_meta(meta: ParseNestedMeta<'_>) -> Result<Self> {
        let mut codec: Option<Type> = None;
        if meta.input.peek(Token![=]) {
            Ok(Self::Text {
                codec: Some(meta.value()?.parse()?),
            })
        } else if meta.input.peek(syn::token::Paren) {
            meta.parse_nested_meta(|meta| {
                if meta.path.is_ident("codec") {
                    if codec.is_some() {
                        return Err(Error::new_spanned(meta.path, "duplicate `codec` key"));
                    }
                    codec = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(Error::new_spanned(meta.path, "unsupported key"))
                }
            })?;
            Ok(Self::Text { codec })
        } else {
            Ok(Self::Text { codec: None })
        }
    }

    /// Parse [`Self`] from a nestd meta, switching on the identifier
    /// of that nested meta.
    fn parse_from_meta(meta: ParseNestedMeta<'_>) -> Result<Self> {
        if meta.path.is_ident("attribute") {
            Self::attribute_from_meta(meta)
        } else if meta.path.is_ident("text") {
            Self::text_from_meta(meta)
        } else {
            Err(Error::new_spanned(meta.path, "unsupported field meta"))
        }
    }

    /// Parse an `#[xml(..)]` meta on a field.
    ///
    /// This switches based on the first identifier within the `#[xml(..)]`
    /// meta and generates an enum variant accordingly.
    ///
    /// Only a single nested meta is allowed; more than one will be
    /// rejected with an appropriate compile-time error.
    ///
    /// If no meta is contained at all, a compile-time error is generated.
    ///
    /// Undefined options or options with incompatible values are rejected
    /// with an appropriate compile-time error.
    pub(crate) fn parse_from_attribute(attr: &Attribute) -> Result<Self> {
        let mut result: Option<Self> = None;

        attr.parse_nested_meta(|meta| {
            if result.is_some() {
                return Err(Error::new_spanned(
                    meta.path,
                    "multiple field type specifiers are not supported",
                ));
            }

            result = Some(Self::parse_from_meta(meta)?);
            Ok(())
        })?;

        if let Some(result) = result {
            Ok(result)
        } else {
            Err(Error::new_spanned(
                attr,
                "missing field type specifier within `#[xml(..)]`",
            ))
        }
    }

    /// Find and parse a `#[xml(..)]` meta on a field.
    ///
    /// This invokes [`Self::parse_from_attribute`] internally on the first
    /// encountered `#[xml(..)]` meta.
    ///
    /// If not exactly one `#[xml(..)]` meta is encountered, an error is
    /// returned. The error is spanned to `err_span`.
    pub(crate) fn parse_from_attributes(attrs: &[Attribute], err_span: &Span) -> Result<Self> {
        let mut result: Option<Self> = None;
        for attr in attrs {
            if !attr.path().is_ident("xml") {
                continue;
            }

            if result.is_some() {
                return Err(Error::new_spanned(
                    attr,
                    "only one #[xml(..)] attribute per field allowed.",
                ));
            }

            result = Some(Self::parse_from_attribute(attr)?);
        }

        if let Some(result) = result {
            Ok(result)
        } else {
            Err(Error::new(*err_span, "missing #[xml(..)] meta on field"))
        }
    }
}
