// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Handling of the insides of compound structures (structs and enum variants)

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::*;

use crate::state::{FromEventsSubmachine, IntoEventsSubmachine, State};
use crate::types::qname_ty;

/// A struct or enum variant's contents.
pub(crate) struct Compound;

impl Compound {
    /// Construct a compound from fields.
    pub(crate) fn from_fields(compound_fields: &Fields) -> Result<Self> {
        match compound_fields {
            Fields::Unit => (),
            other => {
                return Err(Error::new_spanned(
                    other,
                    "cannot derive on non-unit struct (yet!)",
                ))
            }
        }

        Ok(Self)
    }

    /// Make and return a set of states which is used to construct the target
    /// type from XML events.
    ///
    /// The states are returned as partial state machine. See the return
    /// type's documentation for details.
    pub(crate) fn make_from_events_statemachine(
        &self,
        state_ty_ident: &Ident,
        output_cons: &Path,
        state_prefix: &str,
    ) -> Result<FromEventsSubmachine> {
        let default_state_ident = quote::format_ident!("{}Default", state_prefix);
        let builder_data_ident = quote::format_ident!("__data");
        let builder_data_ty: Type = TypePath {
            qself: None,
            path: quote::format_ident!("{}Data{}", state_ty_ident, state_prefix).into(),
        }
        .into();
        let mut states = Vec::new();

        let readable_name = output_cons.to_token_stream().to_string();
        let unknown_attr_err = format!("Unknown attribute in {} element.", readable_name);
        let unknown_child_err = format!("Unknown child in {} element.", readable_name);

        states.push(State::new_with_builder(
            default_state_ident.clone(),
            &builder_data_ident,
            &builder_data_ty,
        ).with_impl(quote! {
            match ev {
                // EndElement in Default state -> done parsing.
                ::xso::exports::rxml::Event::EndElement(_) => {
                    ::core::result::Result::Ok(::std::ops::ControlFlow::Continue(
                        #output_cons
                    ))
                }
                ::xso::exports::rxml::Event::StartElement(..) => {
                    ::core::result::Result::Err(::xso::error::Error::Other(#unknown_child_err))
                }
                ::xso::exports::rxml::Event::Text(..) => {
                    ::core::result::Result::Err(::xso::error::Error::Other("Unexpected text content".into()))
                }
                // we ignore these: a correct parser only generates
                // them at document start, and there we want to indeed
                // not worry about them being in front of the first
                // element.
                ::xso::exports::rxml::Event::XmlDeclaration(_, ::xso::exports::rxml::XmlVersion::V1_0) => ::core::result::Result::Ok(::std::ops::ControlFlow::Break(
                    Self::#default_state_ident { #builder_data_ident }
                ))
            }
        }));

        Ok(FromEventsSubmachine {
            defs: quote! {
                struct #builder_data_ty;
            },
            states,
            init: quote! {
                if attrs.len() > 0 {
                    return ::core::result::Result::Err(::xso::error::Error::Other(
                        #unknown_attr_err,
                    ).into());
                }
                ::core::result::Result::Ok(#state_ty_ident::#default_state_ident {
                    #builder_data_ident: #builder_data_ty,
                })
            },
        })
    }

    /// Make and return a set of states which is used to destructure the
    /// target type into XML events.
    ///
    /// The states are returned as partial state machine. See the return
    /// type's documentation for details.
    ///
    /// **Important:** The returned submachine is not in functional state!
    /// It's `init` must be modified so that a variable called `name` of type
    /// `rxml::QName` is in scope.
    pub(crate) fn make_into_event_iter_statemachine(
        &self,
        input_name: &Path,
        state_prefix: &str,
    ) -> Result<IntoEventsSubmachine> {
        let start_element_state_ident = quote::format_ident!("{}StartElement", state_prefix);
        let end_element_state_ident = quote::format_ident!("{}EndElement", state_prefix);
        let name_ident = quote::format_ident!("name");
        let mut states = Vec::new();

        states.push(
            State::new(start_element_state_ident.clone())
                .with_field(&name_ident, &qname_ty(Span::call_site()))
                .with_impl(quote! {
                    ::core::option::Option::Some(::xso::exports::rxml::Event::StartElement(
                        ::xso::exports::rxml::parser::EventMetrics::zero(),
                        #name_ident,
                        ::xso::exports::rxml::AttrMap::new(),
                    ))
                }),
        );

        states.push(
            State::new(end_element_state_ident.clone()).with_impl(quote! {
                ::core::option::Option::Some(::xso::exports::rxml::Event::EndElement(
                    ::xso::exports::rxml::parser::EventMetrics::zero(),
                ))
            }),
        );

        Ok(IntoEventsSubmachine {
            defs: TokenStream::default(),
            states,
            destructure: quote! {
                #input_name
            },
            init: quote! {
                Self::#start_element_state_ident { #name_ident }
            },
        })
    }
}
