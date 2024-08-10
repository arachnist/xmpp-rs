//! # Generic iterator type implementations
//!
//! This module contains [`AsXml`] iterator implementations for types from
//! foreign libraries (such as the standard library).
//!
//! In order to not clutter the `xso` crate's main namespace, they are
//! stashed away in a separate module.

// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::error::Error;
use crate::rxml_util::Item;
use crate::AsXml;

/// Helper iterator to convert an `Option<T>` to XML.
pub struct OptionAsXml<T: Iterator>(Option<T>);

impl<T: Iterator> OptionAsXml<T> {
    /// Construct a new iterator, wrapping the given iterator.
    ///
    /// If `inner` is `None`, this iterator terminates immediately. Otherwise,
    /// it yields the elements yielded by `inner` until `inner` finishes,
    /// after which this iterator completes, too.
    pub fn new(inner: Option<T>) -> Self {
        Self(inner)
    }
}

impl<'x, T: Iterator<Item = Result<Item<'x>, Error>>> Iterator for OptionAsXml<T> {
    type Item = Result<Item<'x>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.as_mut()?.next()
    }
}

impl<T: AsXml> AsXml for Option<T> {
    type ItemIter<'x> = OptionAsXml<T::ItemIter<'x>> where T: 'x;

    fn as_xml_iter(&self) -> Result<Self::ItemIter<'_>, Error> {
        match self {
            Some(ref value) => Ok(OptionAsXml(Some(T::as_xml_iter(value)?))),
            None => Ok(OptionAsXml(None)),
        }
    }
}

/// Helper iterator to convert an `Box<T>` to XML.
pub struct BoxAsXml<T: Iterator>(Box<T>);

impl<'x, T: Iterator<Item = Result<Item<'x>, Error>>> Iterator for BoxAsXml<T> {
    type Item = Result<Item<'x>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<T: AsXml> AsXml for Box<T> {
    type ItemIter<'x> = BoxAsXml<T::ItemIter<'x>> where T: 'x;

    fn as_xml_iter(&self) -> Result<Self::ItemIter<'_>, Error> {
        Ok(BoxAsXml(Box::new(T::as_xml_iter(&self)?)))
    }
}
