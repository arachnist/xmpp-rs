#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg))]
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

#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;
#[cfg(feature = "std")]
use std::io;

pub mod asxml;
pub mod error;
pub mod fromxml;
#[cfg(feature = "minidom")]
pub mod minidom_compat;
mod rxml_util;
pub mod text;

#[doc(hidden)]
pub mod exports {
    #[cfg(feature = "minidom")]
    pub use minidom;
    pub use rxml;
}

use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    string::String,
    vec::Vec,
};

#[doc(inline)]
pub use text::TextCodec;

#[doc(inline)]
pub use rxml_util::Item;

#[doc = include_str!("from_xml_doc.md")]
#[doc(inline)]
#[cfg(feature = "macros")]
pub use xso_proc::FromXml;

/// # Make a struct or enum serialisable to XML
///
/// This derives the [`AsXml`] trait on a struct or enum. It is the
/// counterpart to [`macro@FromXml`].
///
/// The attributes necessary and available for the derivation to work are
/// documented on [`macro@FromXml`].
#[doc(inline)]
#[cfg(feature = "macros")]
pub use xso_proc::AsXml;

/// Trait allowing to iterate a struct's contents as serialisable
/// [`Item`]s.
///
/// **Important:** Changing the [`ItemIter`][`Self::ItemIter`] associated
/// type is considered a non-breaking change for any given implementation of
/// this trait. Always refer to a type's iterator type using fully-qualified
/// notation, for example: `<T as xso::AsXml>::ItemIter`.
pub trait AsXml {
    /// The iterator type.
    ///
    /// **Important:** Changing this type is considered a non-breaking change
    /// for any given implementation of this trait. Always refer to a type's
    /// iterator type using fully-qualified notation, for example:
    /// `<T as xso::AsXml>::ItemIter`.
    type ItemIter<'x>: Iterator<Item = Result<Item<'x>, self::error::Error>>
    where
        Self: 'x;

    /// Return an iterator which emits the contents of the struct or enum as
    /// serialisable [`Item`] items.
    fn as_xml_iter(&self) -> Result<Self::ItemIter<'_>, self::error::Error>;
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

/// Trait allowing to convert XML text to a value.
///
/// This trait is similar to [`core::str::FromStr`], however, due to
/// restrictions imposed by the orphan rule, a separate trait is needed.
/// Implementations for many standard library types are available. In
/// addition, the following feature flags can enable more implementations:
///
/// - `jid`: `jid::Jid`, `jid::BareJid`, `jid::FullJid`
/// - `uuid`: `uuid::Uuid`
///
/// Because of this unfortunate situation, we are **extremely liberal** with
/// accepting optional dependencies for this purpose. You are very welcome to
/// make merge requests against this crate adding support for parsing
/// third-party crates.
pub trait FromXmlText: Sized {
    /// Convert the given XML text to a value.
    fn from_xml_text(data: String) -> Result<Self, self::error::Error>;
}

impl FromXmlText for String {
    /// Return the string unchanged.
    fn from_xml_text(data: String) -> Result<Self, self::error::Error> {
        Ok(data)
    }
}

impl<T: FromXmlText, B: ToOwned<Owned = T>> FromXmlText for Cow<'_, B> {
    /// Return a [`Cow::Owned`] containing the parsed value.
    fn from_xml_text(data: String) -> Result<Self, self::error::Error> {
        Ok(Cow::Owned(T::from_xml_text(data)?))
    }
}

impl<T: FromXmlText> FromXmlText for Option<T> {
    /// Return a [`Some`] containing the parsed value.
    fn from_xml_text(data: String) -> Result<Self, self::error::Error> {
        Ok(Some(T::from_xml_text(data)?))
    }
}

impl<T: FromXmlText> FromXmlText for Box<T> {
    /// Return a [`Box`] containing the parsed value.
    fn from_xml_text(data: String) -> Result<Self, self::error::Error> {
        Ok(Box::new(T::from_xml_text(data)?))
    }
}

/// Trait to convert a value to an XML text string.
///
/// Implementing this trait for a type allows it to be used both for XML
/// character data within elements and for XML attributes. For XML attributes,
/// the behaviour is defined by [`AsXmlText::as_optional_xml_text`], while
/// XML element text content uses [`AsXmlText::as_xml_text`]. Implementing
/// [`AsXmlText`] automatically provides an implementation of
/// [`AsOptionalXmlText`].
///
/// If your type should only be used in XML attributes and has no correct
/// serialisation in XML text, you should *only* implement
/// [`AsOptionalXmlText`] and omit the [`AsXmlText`] implementation.
///
/// This trait is implemented for many standard library types implementing
/// [`core::fmt::Display`]. In addition, the following feature flags can enable
/// more implementations:
///
/// - `jid`: `jid::Jid`, `jid::BareJid`, `jid::FullJid`
/// - `uuid`: `uuid::Uuid`
///
/// Because of the unfortunate situation as described in [`FromXmlText`], we
/// are **extremely liberal** with accepting optional dependencies for this
/// purpose. You are very welcome to make merge requests against this crate
/// adding support for parsing third-party crates.
pub trait AsXmlText {
    /// Convert the value to an XML string in a context where an absent value
    /// cannot be represented.
    fn as_xml_text(&self) -> Result<Cow<'_, str>, self::error::Error>;

    /// Convert the value to an XML string in a context where an absent value
    /// can be represented.
    ///
    /// The provided implementation will always return the result of
    /// [`Self::as_xml_text`] wrapped into `Some(.)`. By re-implementing
    /// this method, implementors can customize the behaviour for certain
    /// values.
    fn as_optional_xml_text(&self) -> Result<Option<Cow<'_, str>>, self::error::Error> {
        Ok(Some(self.as_xml_text()?))
    }
}

impl AsXmlText for String {
    /// Return the borrowed string contents.
    fn as_xml_text(&self) -> Result<Cow<'_, str>, self::error::Error> {
        Ok(Cow::Borrowed(self.as_str()))
    }
}

impl AsXmlText for str {
    /// Return the borrowed string contents.
    fn as_xml_text(&self) -> Result<Cow<'_, str>, self::error::Error> {
        Ok(Cow::Borrowed(&*self))
    }
}

impl<T: AsXmlText> AsXmlText for Box<T> {
    /// Return the borrowed [`Box`] contents.
    fn as_xml_text(&self) -> Result<Cow<'_, str>, self::error::Error> {
        T::as_xml_text(self)
    }
}

impl<B: AsXmlText + ToOwned> AsXmlText for Cow<'_, B> {
    /// Return the borrowed [`Cow`] contents.
    fn as_xml_text(&self) -> Result<Cow<'_, str>, self::error::Error> {
        B::as_xml_text(self.as_ref())
    }
}

impl<T: AsXmlText> AsXmlText for &T {
    /// Delegate to the `AsXmlText` implementation on `T`.
    fn as_xml_text(&self) -> Result<Cow<'_, str>, self::error::Error> {
        T::as_xml_text(*self)
    }
}

/// Specialized variant of [`AsXmlText`].
///
/// Normally, it should not be necessary to implement this trait as it is
/// automatically implemented for all types implementing [`AsXmlText`].
/// However, if your type can only be serialised as an XML attribute (for
/// example because an absent value has a particular meaning), it is correct
/// to implement [`AsOptionalXmlText`] **instead of** [`AsXmlText`].
///
/// If your type can be serialised as both (text and attribute) but needs
/// special handling in attributes, implement [`AsXmlText`] but provide a
/// custom implementation of [`AsXmlText::as_optional_xml_text`].
pub trait AsOptionalXmlText {
    /// Convert the value to an XML string in a context where an absent value
    /// can be represented.
    fn as_optional_xml_text(&self) -> Result<Option<Cow<'_, str>>, self::error::Error>;
}

impl<T: AsXmlText> AsOptionalXmlText for T {
    fn as_optional_xml_text(&self) -> Result<Option<Cow<'_, str>>, self::error::Error> {
        <Self as AsXmlText>::as_optional_xml_text(self)
    }
}

impl<T: AsXmlText> AsOptionalXmlText for Option<T> {
    fn as_optional_xml_text(&self) -> Result<Option<Cow<'_, str>>, self::error::Error> {
        self.as_ref()
            .map(T::as_optional_xml_text)
            .transpose()
            .map(Option::flatten)
    }
}

/// Control how unknown attributes are handled.
///
/// The variants of this enum are referenced in the
/// `#[xml(on_unknown_attribute = ..)]` which can be used on structs and
/// enum variants. The specified variant controls how attributes, which are
/// not handled by any member of the compound, are handled during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum UnknownAttributePolicy {
    /// All unknown attributes are discarded.
    Discard,

    /// The first unknown attribute which is encountered generates a fatal
    /// parsing error.
    ///
    /// This is the default policy.
    #[default]
    Fail,
}

impl UnknownAttributePolicy {
    #[doc(hidden)]
    /// Implementation of the policy.
    ///
    /// This is an internal API and not subject to semver versioning.
    pub fn apply_policy(&self, msg: &'static str) -> Result<(), self::error::Error> {
        match self {
            Self::Fail => Err(self::error::Error::Other(msg)),
            Self::Discard => Ok(()),
        }
    }
}

/// Control how unknown children are handled.
///
/// The variants of this enum are referenced in the
/// `#[xml(on_unknown_child = ..)]` which can be used on structs and
/// enum variants. The specified variant controls how children, which are not
/// handled by any member of the compound, are handled during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum UnknownChildPolicy {
    /// All unknown children are discarded.
    Discard,

    /// The first unknown child which is encountered generates a fatal
    /// parsing error.
    ///
    /// This is the default policy.
    #[default]
    Fail,
}

impl UnknownChildPolicy {
    #[doc(hidden)]
    /// Implementation of the policy.
    ///
    /// This is an internal API and not subject to semver versioning.
    pub fn apply_policy(&self, msg: &'static str) -> Result<(), self::error::Error> {
        match self {
            Self::Fail => Err(self::error::Error::Other(msg)),
            Self::Discard => Ok(()),
        }
    }
}

/// Attempt to transform a type implementing [`AsXml`] into another
/// type which implements [`FromXml`].
pub fn transform<T: FromXml, F: AsXml>(from: &F) -> Result<T, self::error::Error> {
    let mut iter = self::rxml_util::ItemToEvent::new(from.as_xml_iter()?);
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
        if let Some(v) = sink.feed(event)? {
            return Ok(v);
        }
    }
    Err(self::error::Error::XmlError(rxml::Error::InvalidEof(None)))
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

    let mut iter = from.as_xml_iter()?;
    // consume the element header
    for item in &mut iter {
        let item = item?;
        match item {
            // discard the element header
            Item::XmlDeclaration(..) => (),
            Item::ElementHeadStart(..) => (),
            Item::Attribute(..) => (),
            Item::ElementHeadEnd => {
                // now that the element header is over, we break out
                break;
            }
            Item::Text(..) => panic!("text before end of element header"),
            Item::ElementFoot => panic!("element foot before end of element header"),
        }
    }
    let iter = self::rxml_util::ItemToEvent::new(iter);
    for event in iter {
        let event = event?;
        if let Some(v) = sink.feed(event)? {
            return Ok(v);
        }
    }
    // unreachable! instead of error here, because minidom::Element always
    // produces the complete event sequence of a single element, and FromXml
    // implementations must be constructible from that.
    unreachable!("minidom::Element did not produce enough events to complete element")
}

#[cfg(feature = "std")]
fn map_nonio_error<T>(r: Result<T, io::Error>) -> Result<T, self::error::Error> {
    match r {
        Ok(v) => Ok(v),
        Err(e) => match e.downcast::<rxml::Error>() {
            Ok(e) => Err(e.into()),
            Err(_) => unreachable!("I/O error cannot be caused by &[]"),
        },
    }
}

#[cfg(feature = "std")]
fn read_start_event<I: io::BufRead>(
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
    Err(self::error::Error::XmlError(rxml::Error::InvalidEof(Some(
        rxml::error::ErrorContext::DocumentBegin,
    ))))
}

/// Attempt to parse a type implementing [`FromXml`] from a byte buffer
/// containing XML data.
#[cfg(feature = "std")]
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
        if let Some(v) = builder.feed(map_nonio_error(ev)?)? {
            return Ok(v);
        }
    }
    Err(self::error::Error::XmlError(rxml::Error::InvalidEof(None)))
}

#[cfg(feature = "std")]
fn read_start_event_io<I: io::BufRead>(
    r: &mut rxml::Reader<I>,
) -> io::Result<(rxml::QName, rxml::AttrMap)> {
    for ev in r {
        match ev? {
            rxml::Event::XmlDeclaration(_, rxml::XmlVersion::V1_0) => (),
            rxml::Event::StartElement(_, name, attrs) => return Ok((name, attrs)),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    self::error::Error::Other("Unexpected event at start of document"),
                ))
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        self::error::Error::XmlError(rxml::Error::InvalidEof(Some(
            rxml::error::ErrorContext::DocumentBegin,
        ))),
    ))
}

/// Attempt to parse a type implementing [`FromXml`] from a reader.
#[cfg(feature = "std")]
pub fn from_reader<T: FromXml, R: io::BufRead>(r: R) -> io::Result<T> {
    let mut reader = rxml::Reader::new(r);
    let (name, attrs) = read_start_event_io(&mut reader)?;
    let mut builder = match T::from_events(name, attrs) {
        Ok(v) => v,
        Err(self::error::FromEventsError::Mismatch { .. }) => {
            return Err(self::error::Error::TypeMismatch)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        Err(self::error::FromEventsError::Invalid(e)) => {
            return Err(e).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
    };
    for ev in reader {
        if let Some(v) = builder
            .feed(ev?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        {
            return Ok(v);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        self::error::Error::XmlError(rxml::Error::InvalidEof(None)),
    ))
}

/// Attempt to serialise a type implementing [`AsXml`] to a vector of bytes.
pub fn to_vec<T: AsXml>(xso: &T) -> Result<Vec<u8>, self::error::Error> {
    let iter = xso.as_xml_iter()?;
    let mut writer = rxml::writer::Encoder::new();
    let mut buf = Vec::new();
    for item in iter {
        let item = item?;
        writer.encode(item.as_rxml_item(), &mut buf)?;
    }
    Ok(buf)
}

/// Return true if the string contains exclusively XML whitespace.
///
/// XML whitespace is defined as U+0020 (space), U+0009 (tab), U+000a
/// (newline) and U+000d (carriage return).
pub fn is_xml_whitespace<T: AsRef<[u8]>>(s: T) -> bool {
    s.as_ref()
        .iter()
        .all(|b| *b == b' ' || *b == b'\t' || *b == b'\r' || *b == b'\n')
}
