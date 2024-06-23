// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Identifiers used within generated code.

use proc_macro2::Span;
use syn::*;

/// Container struct for various identifiers used throughout the parser code.
///
/// This struct is passed around from the [`crate::compound::Compound`]
/// downward to the code generators in order to ensure that everyone is on the
/// same page about which identifiers are used for what.
///
/// The recommended usage is to bind the names which are needed into the local
/// scope like this:
///
/// ```text
/// # let scope = FromEventsScope::new();
/// let FromEventsScope {
///     ref attrs,
///     ..
/// } = scope;
/// ```
pub(crate) struct FromEventsScope {
    /// Accesses the `AttrMap` from code in
    /// [`crate::field::FieldBuilderPart::Init`].
    pub(crate) attrs: Ident,
}

impl FromEventsScope {
    /// Create a fresh scope with all necessary identifiers.
    pub(crate) fn new() -> Self {
        // Sadly, `Ident::new` is not `const`, so we have to create even the
        // well-known identifiers from scratch all the time.
        Self {
            attrs: Ident::new("attrs", Span::call_site()),
        }
    }
}

/// Container struct for various identifiers used throughout the generator
/// code.
///
/// This struct is passed around from the [`crate::compound::Compound`]
/// downward to the code generators in order to ensure that everyone is on the
/// same page about which identifiers are used for what.
///
/// See [`FromEventsScope`] for recommendations on the usage.
pub(crate) struct IntoEventsScope {
    /// Accesses the `AttrMap` from code in
    /// [`crate::field::FieldIteratorPart::Header`].
    pub(crate) attrs: Ident,
}

impl IntoEventsScope {
    /// Create a fresh scope with all necessary identifiers.
    pub(crate) fn new() -> Self {
        Self {
            attrs: Ident::new("attrs", Span::call_site()),
        }
    }
}

pub(crate) fn mangle_member(member: &Member) -> Ident {
    match member {
        Member::Named(member) => quote::format_ident!("f{}", member),
        Member::Unnamed(member) => quote::format_ident!("f_u{}", member.index),
    }
}
