// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Compound (struct or enum variant) field types

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, *};

use rxml_validation::NcName;

use crate::error_message::{self, ParentRef};
use crate::meta::{Flag, NameRef, NamespaceRef, XmlFieldMeta};
use crate::scope::{FromEventsScope, IntoEventsScope};
use crate::types::{
    default_fn, from_xml_text_fn, into_optional_xml_text_fn, into_xml_text_fn, string_ty,
    text_codec_decode_fn, text_codec_encode_fn,
};

/// Code slices necessary for declaring and initializing a temporary variable
/// for parsing purposes.
pub(crate) struct FieldTempInit {
    /// The type of the temporary variable.
    pub(crate) ty: Type,

    /// The initializer for the temporary variable.
    pub(crate) init: TokenStream,
}

/// Describe how a struct or enum variant's member is parsed from XML data.
///
/// This struct is returned from [`FieldDef::make_builder_part`] and
/// contains code snippets and instructions for
/// [`Compound::make_from_events_statemachine`][`crate::compound::Compound::make_from_events_statemachine`]
/// to parse the field's data from XML.
pub(crate) enum FieldBuilderPart {
    /// Parse a field from the item's element's start event.
    Init {
        /// Expression and type which extracts the field's data from the
        /// element's start event.
        value: FieldTempInit,
    },

    /// Parse a field from text events.
    Text {
        /// Expression and type which initializes a buffer to use during
        /// parsing.
        value: FieldTempInit,

        /// Statement which takes text and accumulates it into the temporary
        /// value declared via `value`.
        collect: TokenStream,

        /// Expression which evaluates to the field's type, consuming the
        /// temporary value.
        finalize: TokenStream,
    },
}

/// Describe how a struct or enum variant's member is converted to XML data.
///
/// This struct is returned from [`FieldDef::make_iterator_part`] and
/// contains code snippets and instructions for
/// [`Compound::make_into_events_statemachine`][`crate::compound::Compound::make_into_events_statemachine`]
/// to convert the field's data into XML.
pub(crate) enum FieldIteratorPart {
    /// The field is emitted as part of StartElement.
    Header {
        /// A sequence of statements which updates the temporary variables
        /// during the StartElement event's construction, consuming the
        /// field's value.
        setter: TokenStream,
    },

    /// The field is emitted as text event.
    Text {
        /// An expression which consumes the field's value and returns a
        /// String, which is then emitted as text data.
        generator: TokenStream,
    },
}

/// Specify how the field is mapped to XML.
enum FieldKind {
    /// The field maps to an attribute.
    Attribute {
        /// The optional XML namespace of the attribute.
        xml_namespace: Option<NamespaceRef>,

        /// The XML name of the attribute.
        xml_name: NameRef,

        // Flag indicating whether the value should be defaulted if the
        // attribute is absent.
        default_: Flag,
    },

    /// The field maps to the character data of the element.
    Text {
        /// Optional codec to use
        codec: Option<Type>,
    },
}

impl FieldKind {
    /// Construct a new field implementation from the meta attributes.
    ///
    /// `field_ident` is, for some field types, used to infer an XML name if
    /// it is not specified explicitly.
    fn from_meta(meta: XmlFieldMeta, field_ident: Option<&Ident>) -> Result<Self> {
        match meta {
            XmlFieldMeta::Attribute {
                span,
                namespace,
                name,
                default_,
            } => {
                let xml_name = match name {
                    Some(v) => v,
                    None => match field_ident {
                        None => return Err(Error::new(
                            span,
                            "attribute name must be explicitly specified using `#[xml(attribute = ..)] on unnamed fields",
                        )),
                        Some(field_ident) => match NcName::try_from(field_ident.to_string()) {
                            Ok(value) => NameRef::Literal {
                                span: field_ident.span(),
                                value,
                            },
                            Err(e) => {
                                return Err(Error::new(
                                    field_ident.span(),
                                    format!("invalid XML attribute name: {}", e),
                                ))
                            }
                        },
                    }
                };

                Ok(Self::Attribute {
                    xml_name,
                    xml_namespace: namespace,
                    default_,
                })
            }

            XmlFieldMeta::Text { codec } => Ok(Self::Text { codec }),
        }
    }
}

/// Definition of a single field in a compound.
///
/// See [`Compound`][`crate::compound::Compound`] for more information on
/// compounds in general.
pub(crate) struct FieldDef {
    /// The member identifying the field.
    member: Member,

    /// The type of the field.
    ty: Type,

    /// The way the field is mapped to XML.
    kind: FieldKind,
}

impl FieldDef {
    /// Create a new field definition from its declaration.
    ///
    /// The `index` must be the zero-based index of the field even for named
    /// fields.
    pub(crate) fn from_field(field: &syn::Field, index: u32) -> Result<Self> {
        let field_span = field.span();
        let meta = XmlFieldMeta::parse_from_attributes(&field.attrs, &field_span)?;

        let (member, ident) = match field.ident.as_ref() {
            Some(v) => (Member::Named(v.clone()), Some(v)),
            None => (
                Member::Unnamed(Index {
                    index,
                    span: field_span,
                }),
                None,
            ),
        };

        let ty = field.ty.clone();

        Ok(Self {
            member,
            ty,
            kind: FieldKind::from_meta(meta, ident)?,
        })
    }

    /// Access the [`syn::Member`] identifying this field in the original
    /// type.
    pub(crate) fn member(&self) -> &Member {
        &self.member
    }

    /// Access the field's type.
    pub(crate) fn ty(&self) -> &Type {
        &self.ty
    }

    /// Construct the builder pieces for this field.
    ///
    /// `container_name` must be a reference to the compound's type, so that
    /// it can be used for error messages.
    pub(crate) fn make_builder_part(
        &self,
        scope: &FromEventsScope,
        container_name: &ParentRef,
    ) -> Result<FieldBuilderPart> {
        match self.kind {
            FieldKind::Attribute {
                ref xml_name,
                ref xml_namespace,
                ref default_,
            } => {
                let FromEventsScope { ref attrs, .. } = scope;
                let ty = self.ty.clone();

                let missing_msg = error_message::on_missing_attribute(container_name, &self.member);

                let xml_namespace = match xml_namespace {
                    Some(v) => v.to_token_stream(),
                    None => quote! {
                        ::xso::exports::rxml::Namespace::none()
                    },
                };

                let from_xml_text = from_xml_text_fn(ty.clone());

                let on_absent = match default_ {
                    Flag::Absent => quote! {
                        return ::core::result::Result::Err(::xso::error::Error::Other(#missing_msg).into())
                    },
                    Flag::Present(_) => {
                        let default_ = default_fn(ty.clone());
                        quote! {
                            #default_()
                        }
                    }
                };

                Ok(FieldBuilderPart::Init {
                    value: FieldTempInit {
                        init: quote! {
                            match #attrs.remove(#xml_namespace, #xml_name).map(#from_xml_text).transpose()? {
                                ::core::option::Option::Some(v) => v,
                                ::core::option::Option::None => #on_absent,
                            }
                        },
                        ty: self.ty.clone(),
                    },
                })
            }

            FieldKind::Text { ref codec } => {
                let FromEventsScope { ref text, .. } = scope;
                let field_access = scope.access_field(&self.member);
                let finalize = match codec {
                    Some(codec_ty) => {
                        let decode = text_codec_decode_fn(codec_ty.clone(), self.ty.clone());
                        quote! {
                            #decode(#field_access)?
                        }
                    }
                    None => {
                        let from_xml_text = from_xml_text_fn(self.ty.clone());
                        quote! { #from_xml_text(#field_access)? }
                    }
                };

                Ok(FieldBuilderPart::Text {
                    value: FieldTempInit {
                        init: quote! { ::std::string::String::new() },
                        ty: string_ty(Span::call_site()),
                    },
                    collect: quote! {
                        #field_access.push_str(#text.as_str());
                    },
                    finalize,
                })
            }
        }
    }

    /// Construct the iterator pieces for this field.
    ///
    /// `bound_name` must be the name to which the field's value is bound in
    /// the iterator code.
    pub(crate) fn make_iterator_part(
        &self,
        scope: &IntoEventsScope,
        bound_name: &Ident,
    ) -> Result<FieldIteratorPart> {
        match self.kind {
            FieldKind::Attribute {
                ref xml_name,
                ref xml_namespace,
                ..
            } => {
                let IntoEventsScope { ref attrs, .. } = scope;

                let xml_namespace = match xml_namespace {
                    Some(v) => quote! { ::xso::exports::rxml::Namespace::from(#v) },
                    None => quote! {
                        ::xso::exports::rxml::Namespace::NONE
                    },
                };

                let into_optional_xml_text = into_optional_xml_text_fn(self.ty.clone());

                Ok(FieldIteratorPart::Header {
                    // This is a neat little trick:
                    // Option::from(x) converts x to an Option<T> *unless* it
                    // already is an Option<_>.
                    setter: quote! {
                        #into_optional_xml_text(#bound_name)?.and_then(|#bound_name| #attrs.insert(
                            #xml_namespace,
                            #xml_name.to_owned(),
                            #bound_name,
                        ));
                    },
                })
            }

            FieldKind::Text { ref codec } => {
                let generator = match codec {
                    Some(codec_ty) => {
                        let encode = text_codec_encode_fn(codec_ty.clone(), self.ty.clone());
                        quote! { #encode(#bound_name)? }
                    }
                    None => {
                        let into_xml_text = into_xml_text_fn(self.ty.clone());
                        quote! { ::core::option::Option::Some(#into_xml_text(#bound_name)?) }
                    }
                };

                Ok(FieldIteratorPart::Text { generator })
            }
        }
    }

    /// Return true if this field's parsing consumes text data.
    pub(crate) fn is_text_field(&self) -> bool {
        matches!(self.kind, FieldKind::Text { .. })
    }
}
