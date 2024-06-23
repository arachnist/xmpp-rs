// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Handling of structs

use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

use crate::compound::Compound;
use crate::meta::{NameRef, NamespaceRef, XmlCompoundMeta};

/// Parts necessary to construct a `::xso::FromXml` implementation.
pub(crate) struct FromXmlParts {
    /// Additional items necessary for the implementation.
    pub(crate) defs: TokenStream,

    /// The body of the `::xso::FromXml::from_xml` function.
    pub(crate) from_events_body: TokenStream,

    /// The name of the type which is the `::xso::FromXml::Builder`.
    pub(crate) builder_ty_ident: Ident,
}

/// Parts necessary to construct a `::xso::IntoXml` implementation.
pub(crate) struct IntoXmlParts {
    /// Additional items necessary for the implementation.
    pub(crate) defs: TokenStream,

    /// The body of the `::xso::IntoXml::into_event_iter` function.
    pub(crate) into_event_iter_body: TokenStream,

    /// The name of the type which is the `::xso::IntoXml::EventIter`.
    pub(crate) event_iter_ty_ident: Ident,
}

/// Definition of a struct and how to parse it.
pub(crate) struct StructDef {
    /// The XML namespace of the element to map the struct to.
    namespace: NamespaceRef,

    /// The XML name of the element to map the struct to.
    name: NameRef,

    /// The field(s) of this struct.
    inner: Compound,

    /// Name of the target type.
    target_ty_ident: Ident,

    /// Name of the builder type.
    builder_ty_ident: Ident,

    /// Name of the iterator type.
    event_iter_ty_ident: Ident,
}

impl StructDef {
    /// Create a new struct from its name, meta, and fields.
    pub(crate) fn new(ident: &Ident, meta: XmlCompoundMeta, fields: &Fields) -> Result<Self> {
        let Some(namespace) = meta.namespace else {
            return Err(Error::new(meta.span, "`namespace` is required on structs"));
        };

        let Some(name) = meta.name else {
            return Err(Error::new(meta.span, "`name` is required on structs"));
        };

        Ok(Self {
            namespace,
            name,
            inner: Compound::from_fields(fields)?,
            target_ty_ident: ident.clone(),
            builder_ty_ident: quote::format_ident!("{}FromXmlBuilder", ident),
            event_iter_ty_ident: quote::format_ident!("{}IntoXmlIterator", ident),
        })
    }

    pub(crate) fn make_from_events_builder(
        &self,
        vis: &Visibility,
        name_ident: &Ident,
        attrs_ident: &Ident,
    ) -> Result<FromXmlParts> {
        let xml_namespace = &self.namespace;
        let xml_name = &self.name;

        let target_ty_ident = &self.target_ty_ident;
        let builder_ty_ident = &self.builder_ty_ident;
        let state_ty_ident = quote::format_ident!("{}State", builder_ty_ident);

        let defs = self
            .inner
            .make_from_events_statemachine(
                &state_ty_ident,
                &Path::from(target_ty_ident.clone()).into(),
                "Struct",
            )?
            .with_augmented_init(|init| {
                quote! {
                    if name.0 != #xml_namespace || name.1 != #xml_name {
                        ::core::result::Result::Err(::xso::error::FromEventsError::Mismatch {
                            name,
                            attrs,
                        })
                    } else {
                        #init
                    }
                }
            })
            .compile()
            .render(
                vis,
                &builder_ty_ident,
                &state_ty_ident,
                &TypePath {
                    qself: None,
                    path: target_ty_ident.clone().into(),
                }
                .into(),
            )?;

        Ok(FromXmlParts {
            defs,
            from_events_body: quote! {
                #builder_ty_ident::new(#name_ident, #attrs_ident)
            },
            builder_ty_ident: builder_ty_ident.clone(),
        })
    }

    pub(crate) fn make_into_event_iter(&self, vis: &Visibility) -> Result<IntoXmlParts> {
        let xml_namespace = &self.namespace;
        let xml_name = &self.name;

        let target_ty_ident = &self.target_ty_ident;
        let event_iter_ty_ident = &self.event_iter_ty_ident;
        let state_ty_ident = quote::format_ident!("{}State", event_iter_ty_ident);

        let defs = self
            .inner
            .make_into_event_iter_statemachine(&target_ty_ident.clone().into(), "Struct")?
            .with_augmented_init(|init| {
                quote! {
                    let name = (
                        ::xso::exports::rxml::Namespace::from(#xml_namespace),
                        #xml_name.into(),
                    );
                    #init
                }
            })
            .compile()
            .render(
                vis,
                &TypePath {
                    qself: None,
                    path: target_ty_ident.clone().into(),
                }
                .into(),
                &state_ty_ident,
                &event_iter_ty_ident,
            )?;

        Ok(IntoXmlParts {
            defs,
            into_event_iter_body: quote! {
                #event_iter_ty_ident::new(self)
            },
            event_iter_ty_ident: event_iter_ty_ident.clone(),
        })
    }
}
