// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Handling of structs

use proc_macro2::Span;
use quote::quote;
use syn::*;

use crate::common::{AsXmlParts, FromXmlParts, ItemDef};
use crate::compound::Compound;
use crate::meta::{reject_key, Flag, NameRef, NamespaceRef, QNameRef, XmlCompoundMeta};
use crate::types::{ref_ty, ty_from_ident};

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
    item_iter_ty_ident: Ident,

    /// Flag whether debug mode is enabled.
    debug: bool,
}

impl StructDef {
    /// Create a new struct from its name, meta, and fields.
    pub(crate) fn new(ident: &Ident, meta: XmlCompoundMeta, fields: &Fields) -> Result<Self> {
        // We destructure here so that we get informed when new fields are
        // added and can handle them, either by processing them or raising
        // an error if they are present.
        let XmlCompoundMeta {
            span: meta_span,
            qname: QNameRef { namespace, name },
            exhaustive,
            debug,
            builder,
            iterator,
        } = meta;

        reject_key!(exhaustive flag not on "structs" only on "enums");

        let Some(namespace) = namespace else {
            return Err(Error::new(meta_span, "`namespace` is required on structs"));
        };

        let Some(name) = name else {
            return Err(Error::new(meta_span, "`name` is required on structs"));
        };

        let builder_ty_ident = match builder {
            Some(v) => v,
            None => quote::format_ident!("{}FromXmlBuilder", ident.to_string()),
        };

        let item_iter_ty_ident = match iterator {
            Some(v) => v,
            None => quote::format_ident!("{}AsXmlIterator", ident.to_string()),
        };

        Ok(Self {
            inner: Compound::from_fields(fields, &namespace)?,
            namespace,
            name,
            target_ty_ident: ident.clone(),
            builder_ty_ident,
            item_iter_ty_ident,
            debug: debug.is_set(),
        })
    }
}

impl ItemDef for StructDef {
    fn make_from_events_builder(
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
                builder_ty_ident,
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

    fn make_as_xml_iter(&self, vis: &Visibility) -> Result<AsXmlParts> {
        let xml_namespace = &self.namespace;
        let xml_name = &self.name;

        let target_ty_ident = &self.target_ty_ident;
        let item_iter_ty_ident = &self.item_iter_ty_ident;
        let item_iter_ty_lifetime = Lifetime {
            apostrophe: Span::call_site(),
            ident: Ident::new("xso_proc_as_xml_iter_lifetime", Span::call_site()),
        };
        let item_iter_ty = Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments: [PathSegment {
                    ident: item_iter_ty_ident.clone(),
                    arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: token::Lt {
                            spans: [Span::call_site()],
                        },
                        args: [GenericArgument::Lifetime(item_iter_ty_lifetime.clone())]
                            .into_iter()
                            .collect(),
                        gt_token: token::Gt {
                            spans: [Span::call_site()],
                        },
                    }),
                }]
                .into_iter()
                .collect(),
            },
        });
        let state_ty_ident = quote::format_ident!("{}State", item_iter_ty_ident);

        let defs = self
            .inner
            .make_as_item_iter_statemachine(
                &Path::from(target_ty_ident.clone()).into(),
                &state_ty_ident,
                "Struct",
                &item_iter_ty_lifetime,
            )?
            .with_augmented_init(|init| {
                quote! {
                    let name = (
                        ::xso::exports::rxml::Namespace::from(#xml_namespace),
                        ::std::borrow::Cow::Borrowed(#xml_name),
                    );
                    #init
                }
            })
            .compile()
            .render(
                vis,
                &ref_ty(
                    ty_from_ident(target_ty_ident.clone()).into(),
                    item_iter_ty_lifetime.clone(),
                ),
                &state_ty_ident,
                &item_iter_ty_lifetime,
                &item_iter_ty,
            )?;

        Ok(AsXmlParts {
            defs,
            as_xml_iter_body: quote! {
                #item_iter_ty_ident::new(self)
            },
            item_iter_ty,
            item_iter_ty_lifetime,
        })
    }

    fn debug(&self) -> bool {
        self.debug
    }
}
