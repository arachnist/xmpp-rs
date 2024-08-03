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

use crate::compound::Compound;
use crate::error_message::{self, ParentRef};
use crate::meta::{AmountConstraint, Flag, NameRef, NamespaceRef, QNameRef, XmlFieldMeta};
use crate::scope::{AsItemsScope, FromEventsScope};
use crate::types::{
    as_optional_xml_text_fn, as_xml_iter_fn, as_xml_text_fn, default_fn, extend_fn, from_events_fn,
    from_xml_builder_ty, from_xml_text_fn, into_iterator_into_iter_fn, into_iterator_item_ty,
    into_iterator_iter_ty, item_iter_ty, option_ty, ref_ty, string_ty, text_codec_decode_fn,
    text_codec_encode_fn, ty_from_ident,
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
        /// Additional definition items which need to be inserted at module
        /// level for the rest of the implementation to work.
        extra_defs: TokenStream,

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
        /// Additional definition items which need to be inserted at module
        /// level for the rest of the implementation to work.
        extra_defs: TokenStream,

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

        /// Flag indicating whether the value should be defaulted if the
        /// attribute is absent.
        default_: Flag,
    },

    /// The field maps to the character data of the element.
    Text {
        /// Optional codec to use
        codec: Option<Expr>,
    },

    /// The field maps to a child
    Child {
        /// Flag indicating whether the value should be defaulted if the
        /// child is absent.
        default_: Flag,

        /// Number of child elements allowed.
        amount: AmountConstraint,
    },

    /// Extract contents from a child element.
    Extract {
        /// The XML namespace of the child to extract data from.
        xml_namespace: NamespaceRef,

        /// The XML name of the child to extract data from.
        xml_name: NameRef,

        /// Compound which contains the arguments of the `extract(..)` meta
        /// (except the `from`), transformed into a struct with unnamed
        /// fields.
        ///
        /// This is used to generate the parsing/serialisation code, by
        /// essentially "declaring" a shim struct, as if it were a real Rust
        /// struct, and using the result of the parsing process directly for
        /// the field on which the `extract(..)` option was used, instead of
        /// putting it into a Rust struct.
        parts: Compound,
    },
}

fn default_name(span: Span, name: Option<NameRef>, field_ident: Option<&Ident>) -> Result<NameRef> {
    match name {
        Some(v) => Ok(v),
        None => match field_ident {
            None => Err(Error::new(
                span,
                "name must be explicitly specified with the `name` key on unnamed fields",
            )),
            Some(field_ident) => match NcName::try_from(field_ident.to_string()) {
                Ok(value) => Ok(NameRef::Literal {
                    span: field_ident.span(),
                    value,
                }),
                Err(e) => Err(Error::new(
                    field_ident.span(),
                    format!("invalid XML name: {}", e),
                )),
            },
        },
    }
}

impl FieldKind {
    /// Construct a new field implementation from the meta attributes.
    ///
    /// `field_ident` is, for some field types, used to infer an XML name if
    /// it is not specified explicitly.
    ///
    /// `field_ty` is needed for type inferrence on extracted fields.
    ///
    /// `container_namespace` is used in some cases to insert a default
    /// namespace.
    fn from_meta(
        meta: XmlFieldMeta,
        field_ident: Option<&Ident>,
        field_ty: &Type,
        container_namespace: &NamespaceRef,
    ) -> Result<Self> {
        match meta {
            XmlFieldMeta::Attribute {
                span,
                qname: QNameRef { namespace, name },
                default_,
            } => {
                let xml_name = default_name(span, name, field_ident)?;

                Ok(Self::Attribute {
                    xml_name,
                    xml_namespace: namespace,
                    default_,
                })
            }

            XmlFieldMeta::Text { span: _, codec } => Ok(Self::Text { codec }),

            XmlFieldMeta::Child {
                span: _,
                default_,
                amount,
            } => {
                if let Some(AmountConstraint::Any(ref amount_span)) = amount {
                    if let Flag::Present(ref flag_span) = default_ {
                        let mut err = Error::new(
                            *flag_span,
                            "`default` has no meaning for child collections",
                        );
                        err.combine(Error::new(
                            *amount_span,
                            "the field is treated as a collection because of this `n` value",
                        ));
                        return Err(err);
                    }
                }

                Ok(Self::Child {
                    default_,
                    amount: amount.unwrap_or(AmountConstraint::FixedSingle(Span::call_site())),
                })
            }

            XmlFieldMeta::Extract {
                span,
                qname: QNameRef { namespace, name },
                fields,
            } => {
                let xml_namespace = namespace.unwrap_or_else(|| container_namespace.clone());
                let xml_name = default_name(span, name, field_ident)?;

                let field = {
                    let mut fields = fields.into_iter();
                    let Some(field) = fields.next() else {
                        return Err(Error::new(
                            span,
                            "`#[xml(extract(..))]` must contain one `fields(..)` nested meta which contains at least one field meta."
                        ));
                    };

                    if let Some(field) = fields.next() {
                        return Err(Error::new(
                            field.span(),
                            "more than one extracted piece of data is currently not supported",
                        ));
                    }

                    field
                };

                let parts = Compound::from_field_defs(
                    [FieldDef::from_extract(field, 0, field_ty, &xml_namespace)].into_iter(),
                )?;

                Ok(Self::Extract {
                    xml_namespace,
                    xml_name,
                    parts,
                })
            }
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
    pub(crate) fn from_field(
        field: &syn::Field,
        index: u32,
        container_namespace: &NamespaceRef,
    ) -> Result<Self> {
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
            kind: FieldKind::from_meta(meta, ident, &ty, container_namespace)?,
            member,
            ty,
        })
    }

    /// Create a new field definition from its declaration.
    ///
    /// The `index` must be the zero-based index of the field even for named
    /// fields.
    pub(crate) fn from_extract(
        meta: XmlFieldMeta,
        index: u32,
        ty: &Type,
        container_namespace: &NamespaceRef,
    ) -> Result<Self> {
        let span = meta.span();
        Ok(Self {
            member: Member::Unnamed(Index { index, span }),
            ty: ty.clone(),
            kind: FieldKind::from_meta(meta, None, ty, container_namespace)?,
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
                    Some(codec) => {
                        let decode = text_codec_decode_fn(self.ty.clone());
                        quote! {
                            #decode(&#codec, #field_access)?
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

            FieldKind::Child {
                ref default_,
                ref amount,
            } => {
                let FromEventsScope {
                    ref substate_result,
                    ..
                } = scope;
                let field_access = scope.access_field(&self.member);

                let element_ty = match amount {
                    AmountConstraint::FixedSingle(_) => self.ty.clone(),
                    AmountConstraint::Any(_) => into_iterator_item_ty(self.ty.clone()),
                };

                let from_events = from_events_fn(element_ty.clone());
                let from_xml_builder = from_xml_builder_ty(element_ty.clone());

                let matcher = quote! { #from_events(name, attrs) };
                let builder = from_xml_builder;

                match amount {
                    AmountConstraint::FixedSingle(_) => {
                        let missing_msg =
                            error_message::on_missing_child(container_name, &self.member);
                        let duplicate_msg =
                            error_message::on_duplicate_child(container_name, &self.member);

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
                            extra_defs: TokenStream::default(),
                            value: FieldTempInit {
                                init: quote! { ::std::option::Option::None },
                                ty: option_ty(self.ty.clone()),
                            },
                            matcher: quote! {
                                match #matcher {
                                    ::core::result::Result::Ok(v) => if #field_access.is_some() {
                                        ::core::result::Result::Err(::xso::error::FromEventsError::Invalid(::xso::error::Error::Other(#duplicate_msg)))
                                    } else {
                                        ::core::result::Result::Ok(v)
                                    },
                                    ::core::result::Result::Err(e) => ::core::result::Result::Err(e),
                                }
                            },
                            builder,
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
                    AmountConstraint::Any(_) => {
                        let ty_extend = extend_fn(self.ty.clone(), element_ty.clone());
                        let ty_default = default_fn(self.ty.clone());
                        Ok(FieldBuilderPart::Nested {
                            extra_defs: TokenStream::default(),
                            value: FieldTempInit {
                                init: quote! { #ty_default() },
                                ty: self.ty.clone(),
                            },
                            matcher,
                            builder,
                            collect: quote! {
                                #ty_extend(&mut #field_access, [#substate_result]);
                            },
                            finalize: quote! { #field_access },
                        })
                    }
                }
            }

            FieldKind::Extract {
                ref xml_namespace,
                ref xml_name,
                ref parts,
            } => {
                let FromEventsScope {
                    ref substate_result,
                    ..
                } = scope;
                let field_access = scope.access_field(&self.member);

                let missing_msg = error_message::on_missing_child(container_name, &self.member);
                let duplicate_msg = error_message::on_duplicate_child(container_name, &self.member);

                let on_absent = quote! {
                    return ::core::result::Result::Err(::xso::error::Error::Other(#missing_msg).into())
                };

                let from_xml_builder_ty_ident =
                    scope.make_member_type_name(&self.member, "FromXmlBuilder");
                let state_ty_ident = quote::format_ident!("{}State", from_xml_builder_ty_ident,);

                let extra_defs = parts.make_from_events_statemachine(
                    &state_ty_ident,
                    &container_name.child(self.member.clone()),
                    "",
                )?.with_augmented_init(|init| quote! {
                    if name.0 == #xml_namespace && name.1 == #xml_name {
                        #init
                    } else {
                        ::core::result::Result::Err(::xso::error::FromEventsError::Mismatch { name, attrs })
                    }
                }).compile().render(
                    &Visibility::Inherited,
                    &from_xml_builder_ty_ident,
                    &state_ty_ident,
                    &Type::Tuple(TypeTuple {
                        paren_token: token::Paren::default(),
                        elems: [
                            self.ty.clone(),
                        ].into_iter().collect(),
                    })
                )?;
                let from_xml_builder_ty = ty_from_ident(from_xml_builder_ty_ident.clone()).into();

                Ok(FieldBuilderPart::Nested {
                    extra_defs,
                    value: FieldTempInit {
                        init: quote! { ::std::option::Option::None },
                        ty: option_ty(self.ty.clone()),
                    },
                    matcher: quote! {
                        match #state_ty_ident::new(name, attrs) {
                            ::core::result::Result::Ok(v) => if #field_access.is_some() {
                                ::core::result::Result::Err(::xso::error::FromEventsError::Invalid(::xso::error::Error::Other(#duplicate_msg)))
                            } else {
                                ::core::result::Result::Ok(#from_xml_builder_ty_ident(::core::option::Option::Some(v)))
                            },
                            ::core::result::Result::Err(e) => ::core::result::Result::Err(e),
                        }
                    },
                    builder: from_xml_builder_ty,
                    collect: quote! {
                        #field_access = ::std::option::Option::Some(#substate_result.0);
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
        container_name: &ParentRef,
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
                    Some(codec) => {
                        let encode = text_codec_encode_fn(self.ty.clone());
                        quote! { #encode(&#codec, #bound_name)? }
                    }
                    None => {
                        let as_xml_text = as_xml_text_fn(self.ty.clone());
                        quote! { ::core::option::Option::Some(#as_xml_text(#bound_name)?) }
                    }
                };

                Ok(FieldIteratorPart::Text { generator })
            }

            FieldKind::Child {
                default_: _,
                amount: AmountConstraint::FixedSingle(_),
            } => {
                let AsItemsScope { ref lifetime, .. } = scope;

                let as_xml_iter = as_xml_iter_fn(self.ty.clone());
                let item_iter = item_iter_ty(self.ty.clone(), lifetime.clone());

                Ok(FieldIteratorPart::Content {
                    extra_defs: TokenStream::default(),
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

            FieldKind::Child {
                default_: _,
                amount: AmountConstraint::Any(_),
            } => {
                let AsItemsScope { ref lifetime, .. } = scope;

                // This should give us the type of element stored in the
                // collection.
                let element_ty = into_iterator_item_ty(self.ty.clone());

                // And this is the collection type we actually work with --
                // as_xml_iter uses references after all.
                let ty = ref_ty(self.ty.clone(), lifetime.clone());

                // as_xml_iter is called on the bare type (not the ref type)
                let as_xml_iter = as_xml_iter_fn(element_ty.clone());

                // And thus the iterator associated with AsXml is also derived
                // from the bare type.
                let item_iter = item_iter_ty(element_ty.clone(), lifetime.clone());

                // But the iterator for iterating over the elements inside the
                // collection must use the ref type.
                let element_iter = into_iterator_iter_ty(ty.clone());

                // And likewise the into_iter impl.
                let into_iter = into_iterator_into_iter_fn(ty.clone());

                let state_ty = Type::Tuple(TypeTuple {
                    paren_token: token::Paren::default(),
                    elems: [element_iter, option_ty(item_iter)].into_iter().collect(),
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
                                #bound_name.1 = ::core::option::Option::Some(#as_xml_iter(item)?)
                            } else {
                                break ::core::result::Result::Ok(::core::option::Option::None)
                            }
                        }
                    },
                })
            }

            FieldKind::Extract {
                ref xml_namespace,
                ref xml_name,
                ref parts,
            } => {
                let AsItemsScope { ref lifetime, .. } = scope;
                let item_iter_ty_ident = scope.make_member_type_name(&self.member, "AsXmlIterator");
                let state_ty_ident = quote::format_ident!("{}State", item_iter_ty_ident,);
                let mut item_iter_ty = ty_from_ident(item_iter_ty_ident.clone());
                item_iter_ty.path.segments[0].arguments =
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: token::Lt::default(),
                        args: [GenericArgument::Lifetime(lifetime.clone())]
                            .into_iter()
                            .collect(),
                        gt_token: token::Gt::default(),
                    });
                let item_iter_ty = item_iter_ty.into();

                let extra_defs = parts
                    .make_as_item_iter_statemachine(
                        &container_name.child(self.member.clone()),
                        &state_ty_ident,
                        "",
                        lifetime,
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
                        &Visibility::Inherited,
                        &Type::Tuple(TypeTuple {
                            paren_token: token::Paren::default(),
                            elems: [ref_ty(self.ty.clone(), lifetime.clone())]
                                .into_iter()
                                .collect(),
                        }),
                        &state_ty_ident,
                        lifetime,
                        &item_iter_ty,
                    )?;

                Ok(FieldIteratorPart::Content {
                    extra_defs,
                    value: FieldTempInit {
                        init: quote! {
                            #item_iter_ty_ident::new((&#bound_name,))?
                        },
                        ty: item_iter_ty,
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
