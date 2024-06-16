#![forbid(missing_docs, unsafe_code)]
/*!
# XML Streamed Objects -- serde-like parsing for XML

This crate provides the traits for parsing XML data into Rust structs, and
vice versa.

While it is in 0.0.x versions, many features still need to be developed, but
rest assured that there is a solid plan to get it fully usable for even
advanced XML scenarios.

XSO is an acronym for XML Stream(ed) Objects, referring to the main field of
use of this library in parsing XML streams like specified in RFC 6120.
*/
// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
pub mod error;
pub mod minidom_compat;

/// Trait allowing to consume a struct and iterate its contents as
/// serialisable [`rxml::Event`] items.
pub trait IntoXml {
    /// The iterator type.
    type EventIter: Iterator<Item = Result<rxml::Event, self::error::Error>>;

    /// Return an iterator which emits the contents of the struct or enum as
    /// serialisable [`rxml::Event`] items.
    fn into_event_iter(self) -> Result<Self::EventIter, self::error::Error>;
}

/// Trait for a temporary object allowing to construct a struct from
/// [`rxml::Event`] items.
///
/// Objects of this type are generally constructed through
/// [`FromXml::from_events`] and are used to build Rust structs or enums from
/// XML data. The XML data must be fed as `rxml::Event` to the
/// [`feed`][`Self::feed`] method.
pub trait FromEventsBuilder {
    /// The type which will be constructed by this builder.
    type Output;

    /// Feed another [`rxml::Event`] into the element construction
    /// process.
    ///
    /// Once the construction process completes, `Ok(Some(_))` is returned.
    /// When valid data has been fed but more events are needed to fully
    /// construct the resulting struct, `Ok(None)` is returned.
    ///
    /// If the construction fails, `Err(_)` is returned. Errors are generally
    /// fatal and the builder should be assumed to be broken at that point.
    /// Feeding more events after an error may result in panics, errors or
    /// inconsistent result data, though it may never result in unsound or
    /// unsafe behaviour.
    fn feed(&mut self, ev: rxml::Event) -> Result<Option<Self::Output>, self::error::Error>;
}

/// Trait allowing to construct a struct from a stream of
/// [`rxml::Event`] items.
///
/// To use this, first call [`FromXml::from_events`] with the qualified
/// name and the attributes of the corresponding
/// [`rxml::Event::StartElement`] event. If the call succeeds, the
/// returned builder object must be fed with the events representing the
/// contents of the element, and then with the `EndElement` event.
///
/// The `StartElement` passed to `from_events` must not be passed to `feed`.
///
/// **Important:** Changing the [`Builder`][`Self::Builder`] associated type
/// is considered a non-breaking change for any given implementation of this
/// trait. Always refer to a type's builder type using fully-qualified
/// notation, for example: `<T as xso::FromXml>::Builder`.
pub trait FromXml {
    /// A builder type used to construct the element.
    ///
    /// **Important:** Changing this type is considered a non-breaking change
    /// for any given implementation of this trait. Always refer to a type's
    /// builder type using fully-qualified notation, for example:
    /// `<T as xso::FromXml>::Builder`.
    type Builder: FromEventsBuilder<Output = Self>;

    /// Attempt to initiate the streamed construction of this struct from XML.
    ///
    /// If the passed qualified `name` and `attrs` match the element's type,
    /// the [`Self::Builder`] is returned and should be fed with XML events
    /// by the caller.
    ///
    /// Otherwise, an appropriate error is returned.
    fn from_events(
        name: rxml::QName,
        attrs: rxml::AttrMap,
    ) -> Result<Self::Builder, self::error::FromEventsError>;
}

/// Attempt to transform a type implementing [`IntoXml`] into another
/// type which implements [`FromXml`].
pub fn transform<T: FromXml, F: IntoXml>(from: F) -> Result<T, self::error::Error> {
    let mut iter = from.into_event_iter()?;
    let (qname, attrs) = match iter.next() {
        Some(Ok(rxml::Event::StartElement(_, qname, attrs))) => (qname, attrs),
        Some(Err(e)) => return Err(e),
        _ => panic!("into_event_iter did not start with StartElement event!"),
    };
    let mut sink = match T::from_events(qname, attrs) {
        Ok(v) => v,
        Err(self::error::FromEventsError::Mismatch { .. }) => {
            return Err(self::error::Error::TypeMismatch)
        }
        Err(self::error::FromEventsError::Invalid(e)) => return Err(e),
    };
    for event in iter {
        let event = event?;
        match sink.feed(event)? {
            Some(v) => return Ok(v),
            None => (),
        }
    }
    Err(self::error::Error::XmlError(
        rxml::error::XmlError::InvalidEof("during transform"),
    ))
}
