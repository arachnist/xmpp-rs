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
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::*;

mod compound;
mod error_message;
mod field;
mod meta;
mod scope;
mod state;
mod structs;
mod types;

/// Convert an [`syn::Item`] into the parts relevant for us.
///
/// If the item is of an unsupported variant, an appropriate error is
/// returned.
fn parse_struct(item: Item) -> Result<(Visibility, Ident, structs::StructDef)> {
    match item {
        Item::Struct(item) => {
            let meta = meta::XmlCompoundMeta::parse_from_attributes(&item.attrs)?;
            let def = structs::StructDef::new(&item.ident, meta, &item.fields)?;
            Ok((item.vis, item.ident, def))
        }
        other => Err(Error::new_spanned(other, "cannot derive on this item")),
    }
}

/// Generate a `xso::FromXml` implementation for the given item, or fail with
/// a proper compiler error.
fn from_xml_impl(input: Item) -> Result<TokenStream> {
    let (vis, ident, def) = parse_struct(input)?;

    let name_ident = Ident::new("name", Span::call_site());
    let attrs_ident = Ident::new("attrs", Span::call_site());

    let structs::FromXmlParts {
        defs,
        from_events_body,
        builder_ty_ident,
    } = def.make_from_events_builder(&vis, &name_ident, &attrs_ident)?;

    #[cfg_attr(not(feature = "minidom"), allow(unused_mut))]
    let mut result = quote! {
        #defs

        impl ::xso::FromXml for #ident {
            type Builder = #builder_ty_ident;

            fn from_events(
                name: ::xso::exports::rxml::QName,
                attrs: ::xso::exports::rxml::AttrMap,
            ) -> ::core::result::Result<Self::Builder, ::xso::error::FromEventsError> {
                #from_events_body
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
    let (vis, ident, def) = parse_struct(input)?;

    let structs::IntoXmlParts {
        defs,
        into_event_iter_body,
        event_iter_ty_ident,
    } = def.make_into_event_iter(&vis)?;

    #[cfg_attr(not(feature = "minidom"), allow(unused_mut))]
    let mut result = quote! {
        #defs

        impl ::xso::IntoXml for #ident {
            type EventIter = #event_iter_ty_ident;

            fn into_event_iter(self) -> ::core::result::Result<Self::EventIter, ::xso::error::Error> {
                #into_event_iter_body
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
