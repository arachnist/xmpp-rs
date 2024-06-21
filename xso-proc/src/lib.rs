// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(rustdoc::private_intra_doc_links)]
/*!
# Macros for parsing XML into Rust structs, and vice versa

**If you are a user of `xso_proc` or `xso`, please
return to `xso` for more information**. The documentation of
`xso_proc` is geared toward developers of `…_macros` and `…_core`.

**You have been warned.**
*/

// Wondering about RawTokenStream vs. TokenStream?
// syn mostly works with proc_macro2, while the proc macros themselves use
// proc_macro.
use proc_macro::TokenStream as RawTokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

mod meta;

/// Convert an [`syn::Item`] into the parts relevant for us.
///
/// If the item is of an unsupported variant, an appropriate error is
/// returned.
fn parse_struct(item: Item) -> Result<(Visibility, meta::XmlCompoundMeta, Ident)> {
    match item {
        Item::Struct(item) => {
            match item.fields {
                Fields::Unit => (),
                other => {
                    return Err(Error::new_spanned(
                        other,
                        "cannot derive on non-unit struct (yet!)",
                    ))
                }
            }
            let meta = meta::XmlCompoundMeta::parse_from_attributes(&item.attrs)?;
            Ok((item.vis, meta, item.ident))
        }
        other => Err(Error::new_spanned(other, "cannot derive on this item")),
    }
}

/// Generate a `xso::FromXml` implementation for the given item, or fail with
/// a proper compiler error.
fn from_xml_impl(input: Item) -> Result<TokenStream> {
    let (
        vis,
        meta::XmlCompoundMeta {
            namespace,
            name,
            span,
        },
        ident,
    ) = parse_struct(input)?;

    // we rebind to a different name here because otherwise some expressions
    // inside `quote! {}` below get a bit tricky to read (such as
    // `name.1 == #name`).
    let Some(xml_namespace) = namespace else {
        return Err(Error::new(span, "`namespace` key is required"));
    };

    let Some(xml_name) = name else {
        return Err(Error::new(span, "`name` key is required"));
    };

    let from_events_builder_ty_name = quote::format_ident!("{}FromEvents", ident);
    let state_ty_name = quote::format_ident!("{}FromEventsState", ident);

    let unknown_attr_err = format!("Unknown attribute in {} element.", xml_name.value());
    let unknown_child_err = format!("Unknown child in {} element.", xml_name.value());
    let docstr = format!("Build a [`{}`] from XML events", ident);

    #[cfg_attr(not(feature = "minidom"), allow(unused_mut))]
    let mut result = quote! {
        enum #state_ty_name {
            Default,
        }

        #[doc = #docstr]
        #vis struct #from_events_builder_ty_name(::core::option::Option<#state_ty_name>);

        impl ::xso::FromEventsBuilder for #from_events_builder_ty_name {
            type Output = #ident;

            fn feed(
                &mut self,
                ev: ::xso::exports::rxml::Event
            ) -> ::core::result::Result<::core::option::Option<Self::Output>, ::xso::error::Error> {
                match self.0 {
                    ::core::option::Option::None => panic!("feed() called after it returned a non-None value"),
                    ::core::option::Option::Some(#state_ty_name::Default) => match ev {
                        ::xso::exports::rxml::Event::StartElement(..) => {
                            ::core::result::Result::Err(::xso::error::Error::Other(#unknown_child_err))
                        }
                        ::xso::exports::rxml::Event::EndElement(..) => {
                            self.0 = ::core::option::Option::None;
                            ::core::result::Result::Ok(::core::option::Option::Some(#ident))
                        }
                        ::xso::exports::rxml::Event::Text(..) => {
                            ::core::result::Result::Err(::xso::error::Error::Other("Unexpected text content".into()))
                        }
                        // we ignore these: a correct parser only generates
                        // them at document start, and there we want to indeed
                        // not worry about them being in front of the first
                        // element.
                        ::xso::exports::rxml::Event::XmlDeclaration(_, ::xso::exports::rxml::XmlVersion::V1_0) => ::core::result::Result::Ok(::core::option::Option::None)
                    }
                }
            }
        }

        impl ::xso::FromXml for #ident {
            type Builder = #from_events_builder_ty_name;

            fn from_events(
                name: ::xso::exports::rxml::QName,
                attrs: ::xso::exports::rxml::AttrMap,
            ) -> ::core::result::Result<Self::Builder, ::xso::error::FromEventsError> {
                if name.0 != #xml_namespace || name.1 != #xml_name {
                    return ::core::result::Result::Err(::xso::error::FromEventsError::Mismatch { name, attrs });
                }
                if attrs.len() > 0 {
                    return ::core::result::Result::Err(::xso::error::Error::Other(#unknown_attr_err).into());
                }
                ::core::result::Result::Ok(#from_events_builder_ty_name(::core::option::Option::Some(#state_ty_name::Default)))
            }
        }
    };

    #[cfg(feature = "minidom")]
    result.extend(quote! {
        impl ::std::convert::TryFrom<::xso::exports::minidom::Element> for #ident {
            type Error = ::xso::error::FromElementError;

            fn try_from(other: ::xso::exports::minidom::Element) -> ::core::result::Result<Self, Self::Error> {
                ::xso::try_from_element(other)
            }
        }
    });

    Ok(result)
}

/// Macro to derive a `xso::FromXml` implementation on a type.
///
/// The user-facing documentation for this macro lives in the `xso` crate.
#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: RawTokenStream) -> RawTokenStream {
    // Shim wrapper around `from_xml_impl` which converts any errors into
    // actual compiler errors within the resulting token stream.
    let item = syn::parse_macro_input!(input as Item);
    match from_xml_impl(item) {
        Ok(v) => v.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

/// Generate a `xso::IntoXml` implementation for the given item, or fail with
/// a proper compiler error.
fn into_xml_impl(input: Item) -> Result<TokenStream> {
    let (
        vis,
        meta::XmlCompoundMeta {
            namespace,
            name,
            span,
        },
        ident,
    ) = parse_struct(input)?;

    // we rebind to a different name here to stay consistent with
    // `from_xml_impl`.
    let Some(xml_namespace) = namespace else {
        return Err(Error::new(span, "`namespace` key is required"));
    };

    let Some(xml_name) = name else {
        return Err(Error::new(span, "`name` key is required"));
    };

    let into_events_iter_ty_name = quote::format_ident!("{}IntoEvents", ident);
    let state_ty_name = quote::format_ident!("{}IntoEventsState", ident);

    let docstr = format!("Decompose a [`{}`] into XML events", ident);

    #[cfg_attr(not(feature = "minidom"), allow(unused_mut))]
    let mut result = quote! {
        enum #state_ty_name {
            Header,
            Footer,
        }

        #[doc = #docstr]
        #vis struct #into_events_iter_ty_name(::core::option::Option<#state_ty_name>);

        impl ::std::iter::Iterator for #into_events_iter_ty_name {
            type Item = ::core::result::Result<::xso::exports::rxml::Event, ::xso::error::Error>;

            fn next(&mut self) -> ::core::option::Option<Self::Item> {
                match self.0 {
                    ::core::option::Option::Some(#state_ty_name::Header) => {
                        self.0 = ::core::option::Option::Some(#state_ty_name::Footer);
                        ::core::option::Option::Some(::core::result::Result::Ok(::xso::exports::rxml::Event::StartElement(
                            ::xso::exports::rxml::parser::EventMetrics::zero(),
                            (
                                ::xso::exports::rxml::Namespace::from_str(#xml_namespace),
                                match ::xso::exports::rxml::NcName::try_from(#xml_name) {
                                    ::core::result::Result::Ok(v) => v,
                                    ::core::result::Result::Err(e) => {
                                        self.0 = ::core::option::Option::None;
                                        return ::core::option::Option::Some(::core::result::Result::Err(e.into()));

                                    }

                                }
                            ),
                            ::xso::exports::rxml::AttrMap::new(),
                        )))
                    }
                    ::core::option::Option::Some(#state_ty_name::Footer) => {
                        self.0 = ::core::option::Option::None;
                        ::core::option::Option::Some(::core::result::Result::Ok(::xso::exports::rxml::Event::EndElement(
                            ::xso::exports::rxml::parser::EventMetrics::zero(),
                        )))
                    }
                    ::core::option::Option::None => ::core::option::Option::None,
                }
            }
        }

        impl ::xso::IntoXml for #ident {
            type EventIter = #into_events_iter_ty_name;

            fn into_event_iter(self) -> ::core::result::Result<Self::EventIter, ::xso::error::Error> {
                ::core::result::Result::Ok(#into_events_iter_ty_name(::core::option::Option::Some(#state_ty_name::Header)))
            }
        }
    };

    #[cfg(all(feature = "minidom", feature = "panicking-into-impl"))]
    result.extend(quote! {
        impl ::std::convert::From<#ident> for ::xso::exports::minidom::Element {
            fn from(other: #ident) -> Self {
                ::xso::transform(other).expect("seamless conversion into minidom::Element")
            }
        }
    });

    #[cfg(all(feature = "minidom", not(feature = "panicking-into-impl")))]
    result.extend(quote! {
        impl ::std::convert::TryFrom<#ident> for ::xso::exports::minidom::Element {
            type Error = ::xso::error::Error;

            fn try_from(other: #ident) -> ::core::result::Result<Self, Self::Error> {
                ::xso::transform(other)
            }
        }
    });

    Ok(result)
}

/// Macro to derive a `xso::IntoXml` implementation on a type.
///
/// The user-facing documentation for this macro lives in the `xso` crate.
#[proc_macro_derive(IntoXml, attributes(xml))]
pub fn into_xml(input: RawTokenStream) -> RawTokenStream {
    // Shim wrapper around `into_xml_impl` which converts any errors into
    // actual compiler errors within the resulting token stream.
    let item = syn::parse_macro_input!(input as Item);
    match into_xml_impl(item) {
        Ok(v) => v.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
