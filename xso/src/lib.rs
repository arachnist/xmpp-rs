#![forbid(unsafe_code)]
#![warn(missing_docs)]
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
#[cfg(feature = "minidom")]
pub mod minidom_compat;

#[doc(hidden)]
pub mod exports {
    #[cfg(feature = "minidom")]
    pub use minidom;
    pub use rxml;
}

#[doc = include_str!("from_xml_doc.md")]
#[doc(inline)]
#[cfg(feature = "macros")]
pub use xso_proc::FromXml;

/// # Make a struct or enum serialisable to XML
///
/// This derives the [`IntoXml`] trait on a struct or enum. It is the
/// counterpart to [`macro@FromXml`].
///
/// The attributes necessary and available for the derivation to work are
/// documented on [`macro@FromXml`].
#[doc(inline)]
#[cfg(feature = "macros")]
pub use xso_proc::IntoXml;

/// Trait allowing to consume a struct and iterate its contents as
/// serialisable [`rxml::Event`] items.
///
/// **Important:** Changing the [`EventIter`][`Self::EventIter`] associated
/// type is considered a non-breaking change for any given implementation of
/// this trait. Always refer to a type's iterator type using fully-qualified
/// notation, for example: `<T as xso::IntoXml>::EventIter`.
pub trait IntoXml {
    /// The iterator type.
    ///
    /// **Important:** Changing this type is considered a non-breaking change
    /// for any given implementation of this trait. Always refer to a type's
    /// iterator type using fully-qualified notation, for example:
    /// `<T as xso::IntoXml>::EventIter`.
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

/// Attempt to convert a [`minidom::Element`] into a type implementing
/// [`FromXml`], fallably.
///
/// Unlike [`transform`] (which can also be used with an element), this
/// function will return the element unharmed if its element header does not
/// match the expectations of `T`.
#[cfg(feature = "minidom")]
pub fn try_from_element<T: FromXml>(
    from: minidom::Element,
) -> Result<T, self::error::FromElementError> {
    let (qname, attrs) = minidom_compat::make_start_ev_parts(&from)?;
    let mut sink = match T::from_events(qname, attrs) {
        Ok(v) => v,
        Err(self::error::FromEventsError::Mismatch { .. }) => {
            return Err(self::error::FromElementError::Mismatch(from))
        }
        Err(self::error::FromEventsError::Invalid(e)) => {
            return Err(self::error::FromElementError::Invalid(e))
        }
    };

    let mut iter = from.into_event_iter()?;
    iter.next().expect("first event from minidom::Element")?;
    for event in iter {
        let event = event?;
        match sink.feed(event)? {
            Some(v) => return Ok(v),
            None => (),
        }
    }
    // unreachable! instead of error here, because minidom::Element always
    // produces the complete event sequence of a single element, and FromXml
    // implementations must be constructible from that.
    unreachable!("minidom::Element did not produce enough events to complete element")
}

fn map_nonio_error<T>(r: Result<T, rxml::Error>) -> Result<T, self::error::Error> {
    match r {
        Ok(v) => Ok(v),
        Err(rxml::Error::IO(_)) => unreachable!(),
        Err(rxml::Error::Xml(e)) => Err(e.into()),
        Err(rxml::Error::InvalidUtf8Byte(_)) => Err(self::error::Error::Other("invalid utf-8")),
        Err(rxml::Error::InvalidChar(_)) => {
            Err(self::error::Error::Other("non-character encountered"))
        }
        Err(rxml::Error::RestrictedXml(_)) => Err(self::error::Error::Other("restricted xml")),
    }
}

fn read_start_event<I: std::io::BufRead>(
    r: &mut rxml::Reader<I>,
) -> Result<(rxml::QName, rxml::AttrMap), self::error::Error> {
    for ev in r {
        match map_nonio_error(ev)? {
            rxml::Event::XmlDeclaration(_, rxml::XmlVersion::V1_0) => (),
            rxml::Event::StartElement(_, name, attrs) => return Ok((name, attrs)),
            _ => {
                return Err(self::error::Error::Other(
                    "Unexpected event at start of document",
                ))
            }
        }
    }
    Err(self::error::Error::XmlError(
        rxml::error::XmlError::InvalidEof("before start of element"),
    ))
}

/// Attempt to parse a type implementing [`FromXml`] from a byte buffer
/// containing XML data.
pub fn from_bytes<T: FromXml>(mut buf: &[u8]) -> Result<T, self::error::Error> {
    let mut reader = rxml::Reader::new(&mut buf);
    let (name, attrs) = read_start_event(&mut reader)?;
    let mut builder = match T::from_events(name, attrs) {
        Ok(v) => v,
        Err(self::error::FromEventsError::Mismatch { .. }) => {
            return Err(self::error::Error::TypeMismatch)
        }
        Err(self::error::FromEventsError::Invalid(e)) => return Err(e),
    };
    for ev in reader {
        match builder.feed(map_nonio_error(ev)?)? {
            Some(v) => return Ok(v),
            None => (),
        }
    }
    Err(self::error::Error::XmlError(
        rxml::error::XmlError::InvalidEof("while parsing FromXml impl"),
    ))
}
