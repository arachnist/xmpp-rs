// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Handling of the insides of compound structures (structs and enum variants)

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, *};

use crate::error_message::ParentRef;
use crate::field::{FieldBuilderPart, FieldDef, FieldIteratorPart, FieldTempInit, NestedMatcher};
use crate::meta::NamespaceRef;
use crate::scope::{mangle_member, AsItemsScope, FromEventsScope};
use crate::state::{AsItemsSubmachine, FromEventsSubmachine, State};
use crate::types::{
    default_fn, feed_fn, namespace_ty, ncnamestr_cow_ty, phantom_lifetime_ty, ref_ty,
    unknown_attribute_policy_path,
};

fn resolve_policy(policy: Option<Ident>, mut enum_ref: Path) -> Expr {
    match policy {
        Some(ident) => {
            enum_ref.segments.push(ident.into());
            Expr::Path(ExprPath {
                attrs: Vec::new(),
                qself: None,
                path: enum_ref,
            })
        }
        None => {
            let default_fn = default_fn(Type::Path(TypePath {
                qself: None,
                path: enum_ref,
            }));
            Expr::Call(ExprCall {
                attrs: Vec::new(),
                func: Box::new(default_fn),
                paren_token: token::Paren::default(),
                args: punctuated::Punctuated::new(),
            })
        }
    }
}

/// A struct or enum variant's contents.
pub(crate) struct Compound {
    /// The fields of this compound.
    fields: Vec<FieldDef>,

    /// Policy defining how to handle unknown attributes.
    unknown_attribute_policy: Expr,
}

impl Compound {
    /// Construct a compound from processed field definitions.
    pub(crate) fn from_field_defs<I: IntoIterator<Item = Result<FieldDef>>>(
        compound_fields: I,
        unknown_attribute_policy: Option<Ident>,
    ) -> Result<Self> {
        let unknown_attribute_policy = resolve_policy(
            unknown_attribute_policy,
            unknown_attribute_policy_path(Span::call_site()),
        );
        let compound_fields = compound_fields.into_iter();
        let size_hint = compound_fields.size_hint();
        let mut fields = Vec::with_capacity(size_hint.1.unwrap_or(size_hint.0));
        let mut text_field = None;
        for field in compound_fields {
            let field = field?;

            if field.is_text_field() {
                if let Some(other_field) = text_field.as_ref() {
                    let mut err = Error::new_spanned(
                        field.member(),
                        "only one `#[xml(text)]` field allowed per compound",
                    );
                    err.combine(Error::new(
                        *other_field,
                        "the other `#[xml(text)]` field is here",
                    ));
                    return Err(err);
                }
                text_field = Some(field.member().span())
            }

            fields.push(field);
        }
        Ok(Self {
            fields,
            unknown_attribute_policy,
        })
    }

    /// Construct a compound from fields.
    pub(crate) fn from_fields(
        compound_fields: &Fields,
        container_namespace: &NamespaceRef,
        unknown_attribute_policy: Option<Ident>,
    ) -> Result<Self> {
        Self::from_field_defs(
            compound_fields.iter().enumerate().map(|(i, field)| {
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
                FieldDef::from_field(field, index, container_namespace)
            }),
            unknown_attribute_policy,
        )
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
        let scope = FromEventsScope::new(state_ty_ident.clone());
        let FromEventsScope {
            ref attrs,
            ref builder_data_ident,
            ref text,
            ref substate_data,
            ref substate_result,
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
        let mut child_matchers = TokenStream::default();
        let mut fallback_child_matcher = None;
        let mut text_handler = None;
        let mut extra_defs = TokenStream::default();
        let is_tuple = !output_name.is_path();

        for (i, field) in self.fields.iter().enumerate() {
            let member = field.member();
            let builder_field_name = mangle_member(member);
            let part = field.make_builder_part(&scope, output_name)?;
            let state_name = quote::format_ident!("{}Field{}", state_prefix, i);

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

                    if is_tuple {
                        output_cons.extend(quote! {
                            #builder_data_ident.#builder_field_name,
                        });
                    } else {
                        output_cons.extend(quote! {
                            #member: #builder_data_ident.#builder_field_name,
                        });
                    }
                }

                FieldBuilderPart::Text {
                    value: FieldTempInit { ty, init },
                    collect,
                    finalize,
                } => {
                    if text_handler.is_some() {
                        // the existence of only one text handler is enforced
                        // by Compound's constructor(s).
                        panic!("more than one field attempts to collect text data");
                    }

                    builder_data_def.extend(quote! {
                        #builder_field_name: #ty,
                    });
                    builder_data_init.extend(quote! {
                        #builder_field_name: #init,
                    });
                    text_handler = Some(quote! {
                        #collect
                        ::core::result::Result::Ok(::core::ops::ControlFlow::Break(
                            Self::#default_state_ident { #builder_data_ident }
                        ))
                    });

                    if is_tuple {
                        output_cons.extend(quote! {
                            #finalize,
                        });
                    } else {
                        output_cons.extend(quote! {
                            #member: #finalize,
                        });
                    }
                }

                FieldBuilderPart::Nested {
                    extra_defs: field_extra_defs,
                    value: FieldTempInit { ty, init },
                    matcher,
                    builder,
                    collect,
                    finalize,
                } => {
                    let feed = feed_fn(builder.clone());

                    states.push(State::new_with_builder(
                        state_name.clone(),
                        &builder_data_ident,
                        &builder_data_ty,
                    ).with_field(
                        substate_data,
                        &builder,
                    ).with_mut(substate_data).with_impl(quote! {
                        match #feed(&mut #substate_data, ev)? {
                            ::core::option::Option::Some(#substate_result) => {
                                #collect
                                ::core::result::Result::Ok(::core::ops::ControlFlow::Break(Self::#default_state_ident {
                                    #builder_data_ident,
                                }))
                            }
                            ::core::option::Option::None => {
                                ::core::result::Result::Ok(::core::ops::ControlFlow::Break(Self::#state_name {
                                    #builder_data_ident,
                                    #substate_data,
                                }))
                            }
                        }
                    }));

                    builder_data_def.extend(quote! {
                        #builder_field_name: #ty,
                    });

                    builder_data_init.extend(quote! {
                        #builder_field_name: #init,
                    });

                    match matcher {
                        NestedMatcher::Selective(matcher) => {
                            child_matchers.extend(quote! {
                                let (name, attrs) = match #matcher {
                                    ::core::result::Result::Err(::xso::error::FromEventsError::Mismatch { name, attrs }) => (name, attrs),
                                    ::core::result::Result::Err(::xso::error::FromEventsError::Invalid(e)) => return ::core::result::Result::Err(e),
                                    ::core::result::Result::Ok(#substate_data) => {
                                        return ::core::result::Result::Ok(::core::ops::ControlFlow::Break(Self::#state_name {
                                            #builder_data_ident,
                                            #substate_data,
                                        }))
                                    }
                                };
                            });
                        }
                        NestedMatcher::Fallback(matcher) => {
                            if let Some((span, _)) = fallback_child_matcher.as_ref() {
                                let mut err = Error::new(
                                    field.span(),
                                    "more than one field is attempting to consume all unmatched child elements"
                                );
                                err.combine(Error::new(
                                    *span,
                                    "the previous field collecting all unmatched child elements is here"
                                ));
                                return Err(err);
                            }

                            let matcher = quote! {
                                ::core::result::Result::Ok(::core::ops::ControlFlow::Break(Self::#state_name {
                                    #builder_data_ident,
                                    #substate_data: { #matcher },
                                }))
                            };

                            fallback_child_matcher = Some((field.span(), matcher));
                        }
                    }

                    if is_tuple {
                        output_cons.extend(quote! {
                            #finalize,
                        });
                    } else {
                        output_cons.extend(quote! {
                            #member: #finalize,
                        });
                    }

                    extra_defs.extend(field_extra_defs);
                }
            }
        }

        let text_handler = match text_handler {
            Some(v) => v,
            None => quote! {
                // note: u8::is_ascii_whitespace includes U+000C, which is not
                // part of XML's white space definition.'
                if !::xso::is_xml_whitespace(#text.as_bytes()) {
                    ::core::result::Result::Err(::xso::error::Error::Other("Unexpected text content".into()))
                } else {
                    ::core::result::Result::Ok(::core::ops::ControlFlow::Break(
                        Self::#default_state_ident { #builder_data_ident }
                    ))
                }
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
            ParentRef::Unnamed { .. } => {
                quote! {
                    ( #output_cons )
                }
            }
        };

        let child_fallback = match fallback_child_matcher {
            Some((_, matcher)) => matcher,
            None => quote! {
                let _ = (name, attrs);
                ::core::result::Result::Err(::xso::error::Error::Other(#unknown_child_err))
            },
        };

        states.push(State::new_with_builder(
            default_state_ident.clone(),
            builder_data_ident,
            &builder_data_ty,
        ).with_impl(quote! {
            match ev {
                // EndElement in Default state -> done parsing.
                ::xso::exports::rxml::Event::EndElement(_) => {
                    ::core::result::Result::Ok(::core::ops::ControlFlow::Continue(
                        #output_cons
                    ))
                }
                ::xso::exports::rxml::Event::StartElement(_, name, attrs) => {
                    #child_matchers
                    #child_fallback
                }
                ::xso::exports::rxml::Event::Text(_, #text) => {
                    #text_handler
                }
                // we ignore these: a correct parser only generates
                // them at document start, and there we want to indeed
                // not worry about them being in front of the first
                // element.
                ::xso::exports::rxml::Event::XmlDeclaration(_, ::xso::exports::rxml::XmlVersion::V1_0) => ::core::result::Result::Ok(::core::ops::ControlFlow::Break(
                    Self::#default_state_ident { #builder_data_ident }
                ))
            }
        }));

        let unknown_attribute_policy = &self.unknown_attribute_policy;

        Ok(FromEventsSubmachine {
            defs: quote! {
                #extra_defs

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
                    let _: () = #unknown_attribute_policy.apply_policy(#unknown_attr_err)?;
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
    pub(crate) fn make_as_item_iter_statemachine(
        &self,
        input_name: &ParentRef,
        state_ty_ident: &Ident,
        state_prefix: &str,
        lifetime: &Lifetime,
    ) -> Result<AsItemsSubmachine> {
        let scope = AsItemsScope::new(lifetime, state_ty_ident.clone());

        let element_head_start_state_ident =
            quote::format_ident!("{}ElementHeadStart", state_prefix);
        let element_head_end_state_ident = quote::format_ident!("{}ElementHeadEnd", state_prefix);
        let element_foot_state_ident = quote::format_ident!("{}ElementFoot", state_prefix);
        let name_ident = quote::format_ident!("name");
        let ns_ident = quote::format_ident!("ns");
        let dummy_ident = quote::format_ident!("dummy");
        let mut states = Vec::new();

        let is_tuple = !input_name.is_path();
        let mut destructure = TokenStream::default();
        let mut start_init = TokenStream::default();
        let mut extra_defs = TokenStream::default();

        states.push(
            State::new(element_head_start_state_ident.clone())
                .with_field(&dummy_ident, &phantom_lifetime_ty(lifetime.clone()))
                .with_field(&ns_ident, &namespace_ty(Span::call_site()))
                .with_field(
                    &name_ident,
                    &ncnamestr_cow_ty(Span::call_site(), lifetime.clone()),
                ),
        );

        let mut element_head_end_idx = states.len();
        states.push(
            State::new(element_head_end_state_ident.clone()).with_impl(quote! {
                ::core::option::Option::Some(::xso::Item::ElementHeadEnd)
            }),
        );

        for (i, field) in self.fields.iter().enumerate() {
            let member = field.member();
            let bound_name = mangle_member(member);
            let part = field.make_iterator_part(&scope, input_name, &bound_name)?;
            let state_name = quote::format_ident!("{}Field{}", state_prefix, i);
            let ty = scope.borrow(field.ty().clone());

            match part {
                FieldIteratorPart::Header { generator } => {
                    // we have to make sure that we carry our data around in
                    // all the previous states.
                    for state in &mut states[..element_head_end_idx] {
                        state.add_field(&bound_name, &ty);
                    }
                    states.insert(
                        element_head_end_idx,
                        State::new(state_name)
                            .with_field(&bound_name, &ty)
                            .with_impl(quote! {
                                #generator
                            }),
                    );
                    element_head_end_idx += 1;

                    if is_tuple {
                        destructure.extend(quote! {
                            ref #bound_name,
                        });
                    } else {
                        destructure.extend(quote! {
                            #member: ref #bound_name,
                        });
                    }
                    start_init.extend(quote! {
                        #bound_name,
                    });
                }

                FieldIteratorPart::Text { generator } => {
                    // we have to make sure that we carry our data around in
                    // all the previous states.
                    for state in states.iter_mut() {
                        state.add_field(&bound_name, &ty);
                    }
                    states.push(
                        State::new(state_name)
                            .with_field(&bound_name, &ty)
                            .with_impl(quote! {
                                #generator.map(|value| ::xso::Item::Text(
                                    value,
                                ))
                            }),
                    );
                    if is_tuple {
                        destructure.extend(quote! {
                            #bound_name,
                        });
                    } else {
                        destructure.extend(quote! {
                            #member: #bound_name,
                        });
                    }
                    start_init.extend(quote! {
                        #bound_name,
                    });
                }

                FieldIteratorPart::Content {
                    extra_defs: field_extra_defs,
                    value: FieldTempInit { ty, init },
                    generator,
                } => {
                    // we have to make sure that we carry our data around in
                    // all the previous states.
                    for state in states.iter_mut() {
                        state.add_field(&bound_name, &ty);
                    }

                    states.push(
                        State::new(state_name.clone())
                            .with_field(&bound_name, &ty)
                            .with_mut(&bound_name)
                            .with_impl(quote! {
                                #generator?
                            }),
                    );
                    if is_tuple {
                        destructure.extend(quote! {
                            #bound_name,
                        });
                    } else {
                        destructure.extend(quote! {
                            #member: #bound_name,
                        });
                    }
                    start_init.extend(quote! {
                        #bound_name: #init,
                    });

                    extra_defs.extend(field_extra_defs);
                }
            }
        }

        states[0].set_impl(quote! {
            {
                ::core::option::Option::Some(::xso::Item::ElementHeadStart(
                    #ns_ident,
                    #name_ident,
                ))
            }
        });

        states.push(
            State::new(element_foot_state_ident.clone()).with_impl(quote! {
                ::core::option::Option::Some(::xso::Item::ElementFoot)
            }),
        );

        let destructure = match input_name {
            ParentRef::Named(ref input_path) => quote! {
                #input_path { #destructure }
            },
            ParentRef::Unnamed { .. } => quote! {
                ( #destructure )
            },
        };

        Ok(AsItemsSubmachine {
            defs: extra_defs,
            states,
            destructure,
            init: quote! {
                Self::#element_head_start_state_ident { #dummy_ident: ::core::marker::PhantomData, #name_ident: name.1, #ns_ident: name.0, #start_init }
            },
        })
    }

    /// Return a reference to this compound's only field's type.
    ///
    /// If the compound does not have exactly one field, this function returns
    /// None.
    pub(crate) fn single_ty(&self) -> Option<&Type> {
        if self.fields.len() > 1 {
            return None;
        }
        self.fields.get(0).map(|x| x.ty())
    }

    /// Construct a tuple type with this compound's field's types in the same
    /// order as they appear in the compound.
    pub(crate) fn to_tuple_ty(&self) -> TypeTuple {
        TypeTuple {
            paren_token: token::Paren::default(),
            elems: self.fields.iter().map(|x| x.ty().clone()).collect(),
        }
    }

    /// Construct a tuple type with this compound's field's types in the same
    /// order as they appear in the compound.
    pub(crate) fn to_single_or_tuple_ty(&self) -> Type {
        match self.single_ty() {
            None => self.to_tuple_ty().into(),
            Some(v) => v.clone(),
        }
    }

    /// Construct a tuple type with references to this compound's field's
    /// types in the same order as they appear in the compound, with the given
    /// lifetime.
    pub(crate) fn to_ref_tuple_ty(&self, lifetime: &Lifetime) -> TypeTuple {
        TypeTuple {
            paren_token: token::Paren::default(),
            elems: self
                .fields
                .iter()
                .map(|x| ref_ty(x.ty().clone(), lifetime.clone()))
                .collect(),
        }
    }

    /// Return the number of fields in this compound.
    pub(crate) fn field_count(&self) -> usize {
        self.fields.len()
    }
}
