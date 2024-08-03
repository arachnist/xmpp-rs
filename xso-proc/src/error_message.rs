// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Infrastructure for contextual error messages

use std::fmt;

use syn::*;

/// Reference to a compound field's parent
///
/// This reference can be converted to a hopefully-useful human-readable
/// string via [`std::fmt::Display`].
#[derive(Clone, Debug)]
pub(super) enum ParentRef {
    /// The parent is addressable by a path, e.g. a struct type or enum
    /// variant.
    Named(Path),
}

impl From<Path> for ParentRef {
    fn from(other: Path) -> Self {
        Self::Named(other)
    }
}

impl From<&Path> for ParentRef {
    fn from(other: &Path) -> Self {
        Self::Named(other.clone())
    }
}

impl fmt::Display for ParentRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Named(name) => {
                let mut first = true;
                for segment in name.segments.iter() {
                    if !first || name.leading_colon.is_some() {
                        write!(f, "::")?;
                    }
                    first = false;
                    write!(f, "{}", segment.ident)?;
                }
                write!(f, " element")
            }
        }
    }
}

/// Ephemeral struct to create a nice human-readable representation of
/// [`syn::Member`].
///
/// It implements [`std::fmt::Display`] for that purpose and is otherwise of
/// little use.
#[repr(transparent)]
struct FieldName<'x>(&'x Member);

impl fmt::Display for FieldName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Member::Named(v) => write!(f, "field '{}'", v),
            Member::Unnamed(v) => write!(f, "unnamed field {}", v.index),
        }
    }
}

/// Create a string error message for a missing attribute.
///
/// `parent_name` should point at the compound which is being parsed and
/// `field` should be the field to which the attribute belongs.
pub(super) fn on_missing_attribute(parent_name: &ParentRef, field: &Member) -> String {
    format!(
        "Required attribute {} on {} missing.",
        FieldName(field),
        parent_name
    )
}

/// Create a string error message for a missing child element.
///
/// `parent_name` should point at the compound which is being parsed and
/// `field` should be the field to which the child belongs.
pub(super) fn on_missing_child(parent_name: &ParentRef, field: &Member) -> String {
    format!("Missing child {} in {}.", FieldName(&field), parent_name)
}

/// Create a string error message for a duplicate child element.
///
/// `parent_name` should point at the compound which is being parsed and
/// `field` should be the field to which the child belongs.
pub(super) fn on_duplicate_child(parent_name: &ParentRef, field: &Member) -> String {
    format!(
        "{} must not have more than one child in {}.",
        parent_name,
        FieldName(&field)
    )
}
