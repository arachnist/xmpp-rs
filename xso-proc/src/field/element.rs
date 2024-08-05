// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! This module concerns the processing of untyped `minidom::Element`
//! children.
//!
//! In particular, it provides the `#[xml(element)]` implementation.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::*;

use crate::error_message::ParentRef;
use crate::scope::{AsItemsScope, FromEventsScope};
use crate::types::{
    default_fn, element_ty, from_xml_builder_ty, into_iterator_into_iter_fn, into_iterator_iter_ty,
    item_iter_ty, option_ty, ref_ty,
};

use super::{Field, FieldBuilderPart, FieldIteratorPart, FieldTempInit, NestedMatcher};

pub(super) struct ElementField;

impl Field for ElementField {
    fn make_builder_part(
        &self,
        scope: &FromEventsScope,
        _container_name: &ParentRef,
        member: &Member,
        ty: &Type,
    ) -> Result<FieldBuilderPart> {
        let FromEventsScope {
            ref substate_result,
            ..
        } = scope;
        let field_access = scope.access_field(member);

        let element_ty = element_ty(Span::call_site());
        let default_fn = default_fn(ty.clone());
        let builder = from_xml_builder_ty(element_ty.clone());

        Ok(FieldBuilderPart::Nested {
            extra_defs: TokenStream::default(),
            value: FieldTempInit {
                init: quote! { #default_fn() },
                ty: ty.clone(),
            },
            matcher: NestedMatcher::Fallback(quote! {
                #builder::new(name, attrs)
            }),
            builder,
            collect: quote! {
                <#ty as ::core::iter::Extend::<#element_ty>>::extend(&mut #field_access, [#substate_result]);
            },
            finalize: quote! {
                #field_access
            },
        })
    }

    fn make_iterator_part(
        &self,
        scope: &AsItemsScope,
        _container_name: &ParentRef,
        bound_name: &Ident,
        _member: &Member,
        ty: &Type,
    ) -> Result<FieldIteratorPart> {
        let AsItemsScope { ref lifetime, .. } = scope;

        let element_ty = element_ty(Span::call_site());
        let iter_ty = item_iter_ty(element_ty.clone(), lifetime.clone());
        let element_iter = into_iterator_iter_ty(ref_ty(ty.clone(), lifetime.clone()));
        let into_iter = into_iterator_into_iter_fn(ref_ty(ty.clone(), lifetime.clone()));

        let state_ty = Type::Tuple(TypeTuple {
            paren_token: token::Paren::default(),
            elems: [element_iter, option_ty(iter_ty)].into_iter().collect(),
        });

        Ok(FieldIteratorPart::Content {
            extra_defs: TokenStream::default(),
            value: FieldTempInit {
                init: quote! {
                    (#into_iter(#bound_name), ::core::option::Option::None)
                },
                ty: state_ty,
            },
            generator: quote! {
                loop {
                    if let ::core::option::Option::Some(current) = #bound_name.1.as_mut() {
                        if let ::core::option::Option::Some(item) = current.next() {
                            break ::core::option::Option::Some(item).transpose();
                        }
                    }
                    if let ::core::option::Option::Some(item) = #bound_name.0.next() {
                        #bound_name.1 = ::core::option::Option::Some(
                            <#element_ty as ::xso::AsXml>::as_xml_iter(item)?
                        );
                    } else {
                        break ::core::result::Result::Ok(::core::option::Option::None)
                    }
                }
            },
        })
    }
}
