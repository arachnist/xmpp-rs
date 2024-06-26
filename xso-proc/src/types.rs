// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Module with specific [`syn::Type`] constructors.

use proc_macro2::Span;
use syn::{spanned::Spanned, *};

/// Construct a [`syn::Type`] referring to `::xso::exports::rxml::QName`.
pub(crate) fn qname_ty(span: Span) -> Type {
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: Some(syn::token::PathSep {
                spans: [span, span],
            }),
            segments: [
                PathSegment {
                    ident: Ident::new("xso", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("exports", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("rxml", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("QName", span),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
    })
}

/// Construct a [`syn::Expr`] referring to
/// `<#ty as ::xso::FromXmlText>::from_xml_text`.
pub(crate) fn from_xml_text_fn(ty: Type) -> Expr {
    let span = ty.span();
    Expr::Path(ExprPath {
        attrs: Vec::new(),
        qself: Some(QSelf {
            lt_token: syn::token::Lt { spans: [span] },
            ty: Box::new(ty),
            position: 2,
            as_token: Some(syn::token::As { span }),
            gt_token: syn::token::Gt { spans: [span] },
        }),
        path: Path {
            leading_colon: Some(syn::token::PathSep {
                spans: [span, span],
            }),
            segments: [
                PathSegment {
                    ident: Ident::new("xso", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("FromXmlText", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("from_xml_text", span),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
    })
}

/// Construct a [`syn::Expr`] referring to
/// `<#ty as ::xso::IntoOptionalXmlText>::into_optional_xml_text`.
pub(crate) fn into_optional_xml_text_fn(ty: Type) -> Expr {
    let span = ty.span();
    Expr::Path(ExprPath {
        attrs: Vec::new(),
        qself: Some(QSelf {
            lt_token: syn::token::Lt { spans: [span] },
            ty: Box::new(ty),
            position: 2,
            as_token: Some(syn::token::As { span }),
            gt_token: syn::token::Gt { spans: [span] },
        }),
        path: Path {
            leading_colon: Some(syn::token::PathSep {
                spans: [span, span],
            }),
            segments: [
                PathSegment {
                    ident: Ident::new("xso", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("IntoOptionalXmlText", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("into_optional_xml_text", span),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
    })
}

/// Construct a [`syn::Expr`] referring to
/// `<#of_ty as ::std::default::Default>::default`.
pub(crate) fn default_fn(of_ty: Type) -> Expr {
    let span = of_ty.span();
    Expr::Path(ExprPath {
        attrs: Vec::new(),
        qself: Some(QSelf {
            lt_token: syn::token::Lt { spans: [span] },
            ty: Box::new(of_ty),
            position: 3,
            as_token: Some(syn::token::As { span }),
            gt_token: syn::token::Gt { spans: [span] },
        }),
        path: Path {
            leading_colon: Some(syn::token::PathSep {
                spans: [span, span],
            }),
            segments: [
                PathSegment {
                    ident: Ident::new("std", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("default", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("Default", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("default", span),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
    })
}

/// Construct a [`syn::Type`] referring to `::std::string::String`.
pub(crate) fn string_ty(span: Span) -> Type {
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: Some(syn::token::PathSep {
                spans: [span, span],
            }),
            segments: [
                PathSegment {
                    ident: Ident::new("std", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("string", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("String", span),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
    })
}

/// Construct a [`syn::Expr`] referring to
/// `<#ty as ::xso::IntoXmlText>::into_xml_text`.
pub(crate) fn into_xml_text_fn(ty: Type) -> Expr {
    let span = ty.span();
    Expr::Path(ExprPath {
        attrs: Vec::new(),
        qself: Some(QSelf {
            lt_token: syn::token::Lt { spans: [span] },
            ty: Box::new(ty),
            position: 2,
            as_token: Some(syn::token::As { span }),
            gt_token: syn::token::Gt { spans: [span] },
        }),
        path: Path {
            leading_colon: Some(syn::token::PathSep {
                spans: [span, span],
            }),
            segments: [
                PathSegment {
                    ident: Ident::new("xso", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("IntoXmlText", span),
                    arguments: PathArguments::None,
                },
                PathSegment {
                    ident: Ident::new("into_xml_text", span),
                    arguments: PathArguments::None,
                },
            ]
            .into_iter()
            .collect(),
        },
    })
}

/// Construct a [`syn::TypePath`] referring to
/// `<#codec_ty as ::xso::TextCodec::<#for_ty>>` and return the
/// [`syn::Span`] of the `codec_ty` alongside it.
fn text_codec_of(codec_ty: Type, for_ty: Type) -> (Span, TypePath) {
    let span = codec_ty.span();
    (
        span,
        TypePath {
            qself: Some(QSelf {
                lt_token: syn::token::Lt { spans: [span] },
                ty: Box::new(codec_ty),
                position: 2,
                as_token: Some(syn::token::As { span }),
                gt_token: syn::token::Gt { spans: [span] },
            }),
            path: Path {
                leading_colon: Some(syn::token::PathSep {
                    spans: [span, span],
                }),
                segments: [
                    PathSegment {
                        ident: Ident::new("xso", span),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident::new("TextCodec", span),
                        arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            colon2_token: Some(syn::token::PathSep {
                                spans: [span, span],
                            }),
                            lt_token: syn::token::Lt { spans: [span] },
                            args: [GenericArgument::Type(for_ty)].into_iter().collect(),
                            gt_token: syn::token::Gt { spans: [span] },
                        }),
                    },
                ]
                .into_iter()
                .collect(),
            },
        },
    )
}

/// Construct a [`syn::Expr`] referring to
/// `<#codec_ty as ::xso::TextCodec::<#for_ty>>::encode`.
pub(crate) fn text_codec_encode_fn(codec_ty: Type, for_ty: Type) -> Expr {
    let (span, mut ty) = text_codec_of(codec_ty, for_ty);
    ty.path.segments.push(PathSegment {
        ident: Ident::new("encode", span),
        arguments: PathArguments::None,
    });
    Expr::Path(ExprPath {
        attrs: Vec::new(),
        qself: ty.qself,
        path: ty.path,
    })
}

/// Construct a [`syn::Expr`] referring to
/// `<#codec_ty as ::xso::TextCodec::<#for_ty>>::decode`.
pub(crate) fn text_codec_decode_fn(codec_ty: Type, for_ty: Type) -> Expr {
    let (span, mut ty) = text_codec_of(codec_ty, for_ty);
    ty.path.segments.push(PathSegment {
        ident: Ident::new("decode", span),
        arguments: PathArguments::None,
    });
    Expr::Path(ExprPath {
        attrs: Vec::new(),
        qself: ty.qself,
        path: ty.path,
    })
}
