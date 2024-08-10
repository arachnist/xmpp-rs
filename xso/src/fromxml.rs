//! # Generic builder type implementations
//!
//! This module contains [`FromEventsBuilder`] implementations for types from
//! foreign libraries (such as the standard library).
//!
//! In order to not clutter the `xso` crate's main namespace, they are
//! stashed away in a separate module.

// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::error::{Error, FromEventsError};
use crate::{FromEventsBuilder, FromXml};

/// Helper struct to construct an `Option<T>` from XML events.
pub struct OptionBuilder<T: FromEventsBuilder>(T);

impl<T: FromEventsBuilder> FromEventsBuilder for OptionBuilder<T> {
    type Output = Option<T::Output>;

    fn feed(&mut self, ev: rxml::Event) -> Result<Option<Self::Output>, Error> {
        self.0.feed(ev).map(|ok| ok.map(|value| Some(value)))
    }
}

impl<T: FromXml> FromXml for Option<T> {
    type Builder = OptionBuilder<T::Builder>;

    fn from_events(
        name: rxml::QName,
        attrs: rxml::AttrMap,
    ) -> Result<Self::Builder, FromEventsError> {
        Ok(OptionBuilder(T::from_events(name, attrs)?))
    }
}

/// Helper struct to construct an `Box<T>` from XML events.
pub struct BoxBuilder<T: FromEventsBuilder>(Box<T>);

impl<T: FromEventsBuilder> FromEventsBuilder for BoxBuilder<T> {
    type Output = Box<T::Output>;

    fn feed(&mut self, ev: rxml::Event) -> Result<Option<Self::Output>, Error> {
        self.0.feed(ev).map(|ok| ok.map(|value| Box::new(value)))
    }
}

impl<T: FromXml> FromXml for Box<T> {
    type Builder = BoxBuilder<T::Builder>;

    fn from_events(
        name: rxml::QName,
        attrs: rxml::AttrMap,
    ) -> Result<Self::Builder, FromEventsError> {
        Ok(BoxBuilder(Box::new(T::from_events(name, attrs)?)))
    }
}
