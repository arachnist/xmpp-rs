// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Utilities which may eventually move upstream to the `rxml` crate.

use std::borrow::Cow;

use rxml::{Namespace, NcNameStr, XmlVersion};

/// An encodable item.
///
/// Unlike [`rxml::Item`], the contents of this item may either be owned or
/// borrowed, individually. This enables the use in an [`crate::AsXml`] trait
/// even if data needs to be generated during serialisation.
#[derive(Debug)]
pub enum Item<'x> {
    /// XML declaration
    XmlDeclaration(XmlVersion),

    /// Start of an element header
    ElementHeadStart(
        /// Namespace name
        Namespace,
        /// Local name of the attribute
        Cow<'x, NcNameStr>,
    ),

    /// An attribute key/value pair
    Attribute(
        /// Namespace name
        Namespace,
        /// Local name of the attribute
        Cow<'x, NcNameStr>,
        /// Value of the attribute
        Cow<'x, str>,
    ),

    /// End of an element header
    ElementHeadEnd,

    /// A piece of text (in element content, not attributes)
    Text(Cow<'x, str>),

    /// Footer of an element
    ///
    /// This can be used either in places where [`Text`] could be used to
    /// close the most recently opened unclosed element, or it can be used
    /// instead of [`ElementHeadEnd`] to close the element using `/>`, without
    /// any child content.
    ///
    ///   [`Text`]: Self::Text
    ///   [`ElementHeadEnd`]: Self::ElementHeadEnd
    ElementFoot,
}

impl Item<'_> {
    /// Exchange all borrowed pieces inside this item for owned items, cloning
    /// them if necessary.
    pub fn into_owned(self) -> Item<'static> {
        match self {
            Self::XmlDeclaration(v) => Item::XmlDeclaration(v),
            Self::ElementHeadStart(ns, name) => {
                Item::ElementHeadStart(ns, Cow::Owned(name.into_owned()))
            }
            Self::Attribute(ns, name, value) => Item::Attribute(
                ns,
                Cow::Owned(name.into_owned()),
                Cow::Owned(value.into_owned()),
            ),
            Self::ElementHeadEnd => Item::ElementHeadEnd,
            Self::Text(value) => Item::Text(Cow::Owned(value.into_owned())),
            Self::ElementFoot => Item::ElementFoot,
        }
    }

    /// Return an [`rxml::Item`], which borrows data from this item.
    pub fn as_rxml_item<'x>(&'x self) -> rxml::Item<'x> {
        match self {
            Self::XmlDeclaration(ref v) => rxml::Item::XmlDeclaration(*v),
            Self::ElementHeadStart(ref ns, ref name) => rxml::Item::ElementHeadStart(ns, &**name),
            Self::Attribute(ref ns, ref name, ref value) => {
                rxml::Item::Attribute(ns, &**name, &**value)
            }
            Self::ElementHeadEnd => rxml::Item::ElementHeadEnd,
            Self::Text(ref value) => rxml::Item::Text(&**value),
            Self::ElementFoot => rxml::Item::ElementFoot,
        }
    }
}
