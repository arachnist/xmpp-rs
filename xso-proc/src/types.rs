// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Module with specific [`syn::Type`] constructors.

use proc_macro2::Span;
use syn::*;

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
