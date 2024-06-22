// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Handling of structs

use proc_macro2::TokenStream;
use quote::quote;
use syn::*;

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

        match fields {
            Fields::Unit => (),
            other => {
                return Err(Error::new_spanned(
                    other,
                    "cannot derive on non-unit struct (yet!)",
                ))
            }
        }

        Ok(Self {
            namespace,
            name,
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
        let state_ty_name = quote::format_ident!("{}State", builder_ty_ident);

        let unknown_attr_err = format!(
            "Unknown attribute in {} element.",
            xml_name.repr_to_string()
        );
        let unknown_child_err = format!("Unknown child in {} element.", xml_name.repr_to_string());

        let docstr = format!("Build a [`{}`] from XML events", target_ty_ident);

        Ok(FromXmlParts {
            defs: quote! {
                enum #state_ty_name {
                    Default,
                }

                #[doc = #docstr]
                #vis struct #builder_ty_ident(::core::option::Option<#state_ty_name>);

                impl ::xso::FromEventsBuilder for #builder_ty_ident {
                    type Output = #target_ty_ident;

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
                                    ::core::result::Result::Ok(::core::option::Option::Some(#target_ty_ident))
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
            },
            from_events_body: quote! {
                if #name_ident.0 != #xml_namespace || #name_ident.1 != #xml_name {
                    return ::core::result::Result::Err(::xso::error::FromEventsError::Mismatch {
                        name: #name_ident,
                        attrs: #attrs_ident,
                    });
                }
                if attrs.len() > 0 {
                    return ::core::result::Result::Err(::xso::error::Error::Other(
                        #unknown_attr_err,
                    ).into());
                }
                ::core::result::Result::Ok(#builder_ty_ident(::core::option::Option::Some(#state_ty_name::Default)))
            },
            builder_ty_ident: builder_ty_ident.clone(),
        })
    }

    pub(crate) fn make_into_event_iter(&self, vis: &Visibility) -> Result<IntoXmlParts> {
        let xml_namespace = &self.namespace;
        let xml_name = &self.name;

        let target_ty_ident = &self.target_ty_ident;
        let event_iter_ty_ident = &self.event_iter_ty_ident;
        let state_ty_name = quote::format_ident!("{}State", event_iter_ty_ident);

        let docstr = format!("Decompose a [`{}`] into XML events", target_ty_ident);

        Ok(IntoXmlParts {
            defs: quote! {
                enum #state_ty_name {
                    Header,
                    Footer,
                }

                #[doc = #docstr]
                #vis struct #event_iter_ty_ident(::core::option::Option<#state_ty_name>);

                impl ::std::iter::Iterator for #event_iter_ty_ident {
                    type Item = ::core::result::Result<::xso::exports::rxml::Event, ::xso::error::Error>;

                    fn next(&mut self) -> ::core::option::Option<Self::Item> {
                        match self.0 {
                            ::core::option::Option::Some(#state_ty_name::Header) => {
                                self.0 = ::core::option::Option::Some(#state_ty_name::Footer);
                                ::core::option::Option::Some(::core::result::Result::Ok(::xso::exports::rxml::Event::StartElement(
                                    ::xso::exports::rxml::parser::EventMetrics::zero(),
                                    (
                                        ::xso::exports::rxml::Namespace::from_str(#xml_namespace),
                                        #xml_name.to_owned(),
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
            },
            into_event_iter_body: quote! {
                ::core::result::Result::Ok(#event_iter_ty_ident(::core::option::Option::Some(#state_ty_name::Header)))
            },
            event_iter_ty_ident: event_iter_ty_ident.clone(),
        })
    }
}
