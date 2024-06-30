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
use crate::scope::{AsItemsScope, FromEventsScope};
use crate::types::{
    as_optional_xml_text_fn, as_xml_iter_fn, as_xml_text_fn, default_fn, from_events_fn,
    from_xml_builder_ty, from_xml_text_fn, item_iter_ty, option_ty, string_ty,
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

    /// Parse a field from child element events.
    Nested {
        /// Expression and type which initializes a buffer to use during
        /// parsing.
        value: FieldTempInit,

        /// Expression which evaluates to `Result<T, FromEventsError>`,
        /// consuming `name: rxml::QName` and `attrs: rxml::AttrMap`.
        ///
        /// `T` must be the type specified in the
        /// [`Self::Nested::builder`]  field.
        matcher: TokenStream,

        /// Type implementing `xso::FromEventsBuilder` which parses the child
        /// element.
        ///
        /// This type is returned by the expressions in
        /// [`matcher`][`Self::Nested::matcher`].
        builder: Type,

        /// Expression which consumes the value stored in the identifier
        /// [`crate::common::FromEventsScope::substate_result`][`FromEventsScope::substate_result`]
        /// and somehow collects it into the field declared with
        /// [`value`][`Self::Nested::value`].
        collect: TokenStream,

        /// Expression which consumes the data from the field declared with
        /// [`value`][`Self::Nested::value`] and converts it into the field's
        /// type.
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
        /// An expression which consumes the field's value and returns a
        /// `Item`.
        generator: TokenStream,
    },

    /// The field is emitted as text item.
    Text {
        /// An expression which consumes the field's value and returns a
        /// String, which is then emitted as text data.
        generator: TokenStream,
    },

    /// The field is emitted as series of items which form a child element.
    Content {
        /// Expression and type which initializes the nested iterator.
        ///
        /// Note that this is evaluated at construction time of the iterator.
        /// Fields of this variant do not get access to their original data,
        /// unless they carry it in the contents of this `value`.
        value: FieldTempInit,

        /// An expression which uses the value (mutably) and evaluates to
        /// a Result<Option<Item>, Error>. Once the state returns None, the
        /// processing will advance to the next state.
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

    /// The field maps to a child
    Child {
        // Flag indicating whether the value should be defaulted if the
        // child is absent.
        default_: Flag,
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

            XmlFieldMeta::Child { default_ } => Ok(Self::Child { default_ }),
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

            FieldKind::Child { ref default_ } => {
                let FromEventsScope {
                    ref substate_result,
                    ..
                } = scope;
                let field_access = scope.access_field(&self.member);

                let missing_msg = error_message::on_missing_child(container_name, &self.member);

                let from_events = from_events_fn(self.ty.clone());
                let from_xml_builder = from_xml_builder_ty(self.ty.clone());

                let on_absent = match default_ {
                    Flag::Absent => quote! {
                        return ::core::result::Result::Err(::xso::error::Error::Other(#missing_msg).into())
                    },
                    Flag::Present(_) => {
                        let default_ = default_fn(self.ty.clone());
                        quote! {
                            #default_()
                        }
                    }
                };

                Ok(FieldBuilderPart::Nested {
                    value: FieldTempInit {
                        init: quote! { ::std::option::Option::None },
                        ty: option_ty(self.ty.clone()),
                    },
                    matcher: quote! {
                        #from_events(name, attrs)
                    },
                    builder: from_xml_builder,
                    collect: quote! {
                        #field_access = ::std::option::Option::Some(#substate_result);
                    },
                    finalize: quote! {
                        match #field_access {
                            ::std::option::Option::Some(value) => value,
                            ::std::option::Option::None => #on_absent,
                        }
                    },
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
        scope: &AsItemsScope,
        bound_name: &Ident,
    ) -> Result<FieldIteratorPart> {
        match self.kind {
            FieldKind::Attribute {
                ref xml_name,
                ref xml_namespace,
                ..
            } => {
                let xml_namespace = match xml_namespace {
                    Some(v) => quote! { ::xso::exports::rxml::Namespace::from(#v) },
                    None => quote! {
                        ::xso::exports::rxml::Namespace::NONE
                    },
                };

                let as_optional_xml_text = as_optional_xml_text_fn(self.ty.clone());

                Ok(FieldIteratorPart::Header {
                    generator: quote! {
                        #as_optional_xml_text(#bound_name)?.map(|#bound_name| ::xso::Item::Attribute(
                            #xml_namespace,
                            ::std::borrow::Cow::Borrowed(#xml_name),
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
                        let as_xml_text = as_xml_text_fn(self.ty.clone());
                        quote! { ::core::option::Option::Some(#as_xml_text(#bound_name)?) }
                    }
                };

                Ok(FieldIteratorPart::Text { generator })
            }

            FieldKind::Child { default_: _ } => {
                let AsItemsScope { ref lifetime, .. } = scope;

                let as_xml_iter = as_xml_iter_fn(self.ty.clone());
                let item_iter = item_iter_ty(self.ty.clone(), lifetime.clone());

                Ok(FieldIteratorPart::Content {
                    value: FieldTempInit {
                        init: quote! {
                            #as_xml_iter(#bound_name)?
                        },
                        ty: item_iter,
                    },
                    generator: quote! {
                        #bound_name.next().transpose()
                    },
                })
            }
        }
    }

    /// Return true if this field's parsing consumes text data.
    pub(crate) fn is_text_field(&self) -> bool {
        matches!(self.kind, FieldKind::Text { .. })
    }
}
