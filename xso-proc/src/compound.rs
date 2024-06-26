// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Handling of the insides of compound structures (structs and enum variants)

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::*;

use crate::error_message::ParentRef;
use crate::field::{FieldBuilderPart, FieldDef, FieldIteratorPart, FieldTempInit};
use crate::scope::{mangle_member, FromEventsScope, IntoEventsScope};
use crate::state::{FromEventsSubmachine, IntoEventsSubmachine, State};
use crate::types::qname_ty;

/// A struct or enum variant's contents.
pub(crate) struct Compound {
    /// The fields of this compound.
    fields: Vec<FieldDef>,
}

impl Compound {
    /// Construct a compound from fields.
    pub(crate) fn from_fields(compound_fields: &Fields) -> Result<Self> {
        let mut fields = Vec::with_capacity(compound_fields.len());
        for (i, field) in compound_fields.iter().enumerate() {
            let index = match i.try_into() {
                Ok(v) => v,
                // we are converting to u32, are you crazy?!
                // (u32, because syn::Member::Index needs that.)
                Err(_) => {
                    return Err(Error::new_spanned(
                        field,
                        "okay, mate, that are way too many fields. get your life together.",
                    ))
                }
            };
            fields.push(FieldDef::from_field(field, index)?);
        }

        Ok(Self { fields })
    }

    /// Make and return a set of states which is used to construct the target
    /// type from XML events.
    ///
    /// The states are returned as partial state machine. See the return
    /// type's documentation for details.
    pub(crate) fn make_from_events_statemachine(
        &self,
        state_ty_ident: &Ident,
        output_name: &ParentRef,
        state_prefix: &str,
    ) -> Result<FromEventsSubmachine> {
        let scope = FromEventsScope::new();
        let FromEventsScope {
            ref attrs,
            ref builder_data_ident,
            ref text,
            ..
        } = scope;

        let default_state_ident = quote::format_ident!("{}Default", state_prefix);
        let builder_data_ty: Type = TypePath {
            qself: None,
            path: quote::format_ident!("{}Data{}", state_ty_ident, state_prefix).into(),
        }
        .into();
        let mut states = Vec::new();

        let mut builder_data_def = TokenStream::default();
        let mut builder_data_init = TokenStream::default();
        let mut output_cons = TokenStream::default();
        let mut text_handler = None;

        for field in self.fields.iter() {
            let member = field.member();
            let builder_field_name = mangle_member(member);
            let part = field.make_builder_part(&scope, &output_name)?;

            match part {
                FieldBuilderPart::Init {
                    value: FieldTempInit { ty, init },
                } => {
                    builder_data_def.extend(quote! {
                        #builder_field_name: #ty,
                    });

                    builder_data_init.extend(quote! {
                        #builder_field_name: #init,
                    });

                    output_cons.extend(quote! {
                        #member: #builder_data_ident.#builder_field_name,
                    });
                }

                FieldBuilderPart::Text {
                    value: FieldTempInit { ty, init },
                    collect,
                    finalize,
                } => {
                    if text_handler.is_some() {
                        return Err(Error::new_spanned(
                            field.member(),
                            "more than one field attempts to collect text data",
                        ));
                    }

                    builder_data_def.extend(quote! {
                        #builder_field_name: #ty,
                    });
                    builder_data_init.extend(quote! {
                        #builder_field_name: #init,
                    });
                    text_handler = Some(quote! {
                        #collect
                        ::core::result::Result::Ok(::std::ops::ControlFlow::Break(
                            Self::#default_state_ident { #builder_data_ident }
                        ))
                    });
                    output_cons.extend(quote! {
                        #member: #finalize,
                    });
                }
            }
        }

        let text_handler = match text_handler {
            Some(v) => v,
            None => quote! {
                ::core::result::Result::Err(::xso::error::Error::Other("Unexpected text content".into()))
            },
        };

        let unknown_attr_err = format!("Unknown attribute in {}.", output_name);
        let unknown_child_err = format!("Unknown child in {}.", output_name);

        let output_cons = match output_name {
            ParentRef::Named(ref path) => {
                quote! {
                    #path { #output_cons }
                }
            }
        };

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
                ::xso::exports::rxml::Event::Text(_, #text) => {
                    #text_handler
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
                struct #builder_data_ty {
                    #builder_data_def
                }
            },
            states,
            init: quote! {
                let #builder_data_ident = #builder_data_ty {
                    #builder_data_init
                };
                if #attrs.len() > 0 {
                    return ::core::result::Result::Err(::xso::error::Error::Other(
                        #unknown_attr_err,
                    ).into());
                }
                ::core::result::Result::Ok(#state_ty_ident::#default_state_ident { #builder_data_ident })
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
        let scope = IntoEventsScope::new();
        let IntoEventsScope { ref attrs, .. } = scope;

        let start_element_state_ident = quote::format_ident!("{}StartElement", state_prefix);
        let end_element_state_ident = quote::format_ident!("{}EndElement", state_prefix);
        let name_ident = quote::format_ident!("name");
        let mut states = Vec::new();

        let mut init_body = TokenStream::default();
        let mut destructure = TokenStream::default();
        let mut start_init = TokenStream::default();

        states.push(
            State::new(start_element_state_ident.clone())
                .with_field(&name_ident, &qname_ty(Span::call_site())),
        );

        for (i, field) in self.fields.iter().enumerate() {
            let member = field.member();
            let bound_name = mangle_member(member);
            let part = field.make_iterator_part(&scope, &bound_name)?;
            let state_name = quote::format_ident!("{}Field{}", state_prefix, i);

            match part {
                FieldIteratorPart::Header { setter } => {
                    destructure.extend(quote! {
                        #member: #bound_name,
                    });
                    init_body.extend(setter);
                    start_init.extend(quote! {
                        #bound_name,
                    });
                    states[0].add_field(&bound_name, field.ty());
                }

                FieldIteratorPart::Text { generator } => {
                    // we have to make sure that we carry our data around in
                    // all the previous states.
                    for state in states.iter_mut() {
                        state.add_field(&bound_name, field.ty());
                    }
                    states.push(
                        State::new(state_name)
                            .with_field(&bound_name, field.ty())
                            .with_impl(quote! {
                                ::core::option::Option::Some(::xso::exports::rxml::Event::Text(
                                    ::xso::exports::rxml::parser::EventMetrics::zero(),
                                    #generator,
                                ))
                            }),
                    );
                    destructure.extend(quote! {
                        #member: #bound_name,
                    });
                    start_init.extend(quote! {
                        #bound_name,
                    });
                }
            }
        }

        states[0].set_impl(quote! {
            {
                let mut #attrs = ::xso::exports::rxml::AttrMap::new();
                #init_body
                ::core::option::Option::Some(::xso::exports::rxml::Event::StartElement(
                    ::xso::exports::rxml::parser::EventMetrics::zero(),
                    #name_ident,
                    #attrs,
                ))
            }
        });

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
                #input_name { #destructure }
            },
            init: quote! {
                Self::#start_element_state_ident { #name_ident, #start_init }
            },
        })
    }
}
