//! Implementations of traits from this crate for minidom types

// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    vec::IntoIter,
};
use core::marker::PhantomData;

use minidom::{Element, Node};

use rxml::{
    parser::EventMetrics,
    writer::{SimpleNamespaces, TrackNamespace},
    AttrMap, Event, Name, NameStr, Namespace, NcName, NcNameStr,
};

use crate::{
    error::{Error, FromEventsError},
    rxml_util::{EventToItem, Item},
    AsXml, FromEventsBuilder, FromXml,
};

/// State machine for converting a minidom Element into rxml events.
enum IntoEventsInner {
    /// Element header: the element is still intact and we need to generate
    /// the [`rxml::Event::StartElement`] event from the namespace, name, and
    /// attributes.
    Header(Element),

    /// Content: The contents of the element are streamed as events.
    Nodes {
        /// Remaining child nodes (text and/or children) to emit.
        remaining: IntoIter<Node>,

        /// When emitting a child element, this is a nested [`IntoEvents`]
        /// instance for that child element.
        nested: Option<Box<IntoEvents>>,
    },

    /// End of iteration: this state generates an end-of-iterator state.
    ///
    /// Note that the [`rxml::Event::EndElement`] event for the element itself
    /// is generated by the iterator already in the `Nodes` state, when
    /// `nested` is None and `remaining` returns `None` from its `next()`
    /// implementation.
    Fin,
}

/// Create the parts for a [`rxml::Event::StartElement`] from a
/// [`minidom::Element`].
///
/// Note that this copies the attribute data as well as namespace and name.
/// This is due to limitations in the [`minidom::Element`] API.
// NOTE to developers: The limitations are not fully trivial to overcome:
// the attributes use a BTreeMap internally, which does not offer a `drain`
// iterator.
pub fn make_start_ev_parts(el: &Element) -> Result<(rxml::QName, AttrMap), Error> {
    let name = NcName::try_from(el.name())?;
    let namespace = Namespace::from(el.ns());

    let mut attrs = AttrMap::new();
    for (name, value) in el.attrs() {
        let name = Name::try_from(name)?;
        let (prefix, name) = name.split_name()?;
        let namespace = if let Some(prefix) = prefix {
            if prefix == "xml" {
                Namespace::XML
            } else {
                let ns = match el.prefixes.get(&Some(prefix.into())) {
                    Some(v) => v,
                    None => {
                        panic!("undeclared xml namespace prefix in minidom::Element")
                    }
                };
                Namespace::from(ns.to_owned())
            }
        } else {
            Namespace::NONE
        };

        attrs.insert(namespace, name, value.to_owned());
    }

    Ok(((namespace, name), attrs))
}

impl IntoEventsInner {
    fn next(&mut self) -> Result<Option<Event>, Error> {
        match self {
            IntoEventsInner::Header(ref mut el) => {
                let (qname, attrs) = make_start_ev_parts(el)?;
                let event = Event::StartElement(EventMetrics::zero(), qname, attrs);

                *self = IntoEventsInner::Nodes {
                    remaining: el.take_nodes().into_iter(),
                    nested: None,
                };
                Ok(Some(event))
            }
            IntoEventsInner::Nodes {
                ref mut nested,
                ref mut remaining,
            } => {
                loop {
                    if let Some(nested) = nested.as_mut() {
                        if let Some(ev) = nested.next() {
                            return Some(ev).transpose();
                        }
                    }
                    match remaining.next() {
                        Some(Node::Text(text)) => {
                            return Ok(Some(Event::Text(EventMetrics::zero(), text)));
                        }
                        Some(Node::Element(el)) => {
                            *nested = Some(Box::new(IntoEvents::new(el)));
                            // fallthrough to next loop iteration
                        }
                        None => {
                            // end of element, switch state and emit EndElement
                            *self = IntoEventsInner::Fin;
                            return Ok(Some(Event::EndElement(EventMetrics::zero())));
                        }
                    }
                }
            }
            IntoEventsInner::Fin => Ok(None),
        }
    }
}

/// Convert a [`minidom::Element`] into [`rxml::Event`]s.
///
/// This can be constructed from the
/// [`IntoXml::into_event_iter`][`crate::IntoXml::into_event_iter`]
/// implementation on [`minidom::Element`].
struct IntoEvents(IntoEventsInner);

impl IntoEvents {
    fn new(el: Element) -> Self {
        IntoEvents(IntoEventsInner::Header(el))
    }
}

impl Iterator for IntoEvents {
    type Item = Result<Event, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().transpose()
    }
}

enum AsXmlState<'a> {
    /// Element header: we need to generate the
    /// [`Item::ElementHeadStart`] item from the namespace and name.
    Header { element: &'a Element },

    /// Element header: we now generate the attributes.
    Attributes {
        /// The element (needed for the contents later and to access the
        /// prefix mapping).
        element: &'a Element,

        /// Attribute iterator.
        attributes: minidom::element::Attrs<'a>,
    },

    /// Content: The contents of the element are streamed as events.
    Nodes {
        /// Remaining child nodes (text and/or children) to emit.
        nodes: minidom::element::Nodes<'a>,

        /// When emitting a child element, this is a nested [`IntoEvents`]
        /// instance for that child element.
        nested: Option<Box<ElementAsXml<'a>>>,
    },
}

/// Convert a [`minidom::Element`] to [`Item`][`crate::rxml_util::Item`]s.
///
/// This can be constructed from the
/// [`AsXml::as_xml_iter`][`crate::AsXml::as_xml_iter`]
/// implementation on [`minidom::Element`].
pub struct ElementAsXml<'a>(Option<AsXmlState<'a>>);

impl<'a> Iterator for ElementAsXml<'a> {
    type Item = Result<Item<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            None => None,
            Some(AsXmlState::Header { ref element }) => {
                let item = Item::ElementHeadStart(
                    Namespace::from(element.ns()),
                    Cow::Borrowed(match <&NcNameStr>::try_from(element.name()) {
                        Ok(v) => v,
                        Err(e) => {
                            self.0 = None;
                            return Some(Err(e.into()));
                        }
                    }),
                );
                self.0 = Some(AsXmlState::Attributes {
                    element,
                    attributes: element.attrs(),
                });
                Some(Ok(item))
            }
            Some(AsXmlState::Attributes {
                ref mut attributes,
                ref element,
            }) => {
                if let Some((name, value)) = attributes.next() {
                    let name = match <&NameStr>::try_from(name) {
                        Ok(v) => v,
                        Err(e) => {
                            self.0 = None;
                            return Some(Err(e.into()));
                        }
                    };
                    let (prefix, name) = match name.split_name() {
                        Ok(v) => v,
                        Err(e) => {
                            self.0 = None;
                            return Some(Err(e.into()));
                        }
                    };
                    let namespace = if let Some(prefix) = prefix {
                        if prefix == "xml" {
                            Namespace::XML
                        } else {
                            let ns = match element.prefixes.get(&Some(prefix.as_str().to_owned())) {
                                Some(v) => v,
                                None => {
                                    panic!("undeclared xml namespace prefix in minidom::Element")
                                }
                            };
                            Namespace::from(ns.to_owned())
                        }
                    } else {
                        Namespace::NONE
                    };
                    Some(Ok(Item::Attribute(
                        namespace,
                        Cow::Borrowed(name),
                        Cow::Borrowed(value),
                    )))
                } else {
                    self.0 = Some(AsXmlState::Nodes {
                        nodes: element.nodes(),
                        nested: None,
                    });
                    Some(Ok(Item::ElementHeadEnd))
                }
            }
            Some(AsXmlState::Nodes {
                ref mut nodes,
                ref mut nested,
            }) => {
                if let Some(nested) = nested.as_mut() {
                    if let Some(next) = nested.next() {
                        return Some(next);
                    }
                }
                *nested = None;
                match nodes.next() {
                    None => {
                        self.0 = None;
                        Some(Ok(Item::ElementFoot))
                    }
                    Some(minidom::Node::Text(ref text)) => {
                        Some(Ok(Item::Text(Cow::Borrowed(text))))
                    }
                    Some(minidom::Node::Element(ref element)) => {
                        let mut iter = match element.as_xml_iter() {
                            Ok(v) => v,
                            Err(e) => {
                                self.0 = None;
                                return Some(Err(e.into()));
                            }
                        };
                        let item = iter.next().unwrap();
                        *nested = Some(Box::new(iter));
                        Some(item)
                    }
                }
            }
        }
    }
}

impl AsXml for minidom::Element {
    type ItemIter<'a> = ElementAsXml<'a>;

    fn as_xml_iter(&self) -> Result<Self::ItemIter<'_>, Error> {
        Ok(ElementAsXml(Some(AsXmlState::Header { element: self })))
    }
}

/// Construct a [`minidom::Element`] from [`rxml::Event`]s
///
/// This can be constructed from the
/// [`FromXml::from_events`][`crate::FromXml::from_events`]
/// implementation on [`minidom::Element`].
pub struct ElementFromEvents {
    inner: Option<Element>,
    nested: Option<Box<ElementFromEvents>>,
}

impl ElementFromEvents {
    /// Construct a new builder from an element header.
    ///
    /// Unlike the [`FromXml::from_events`] implementation on
    /// [`minidom::Element`], this is contractually infallible. Using this may
    /// thus save you an `unwrap()` call.
    pub fn new(qname: rxml::QName, attrs: rxml::AttrMap) -> Self {
        let mut prefixes = SimpleNamespaces::new();
        let mut builder = Element::builder(qname.1, qname.0);
        for ((namespace, name), value) in attrs.into_iter() {
            if namespace.is_none() {
                builder = builder.attr(name, value);
            } else {
                let (is_new, prefix) = prefixes.declare_with_auto_prefix(namespace.clone());
                let name = prefix.with_suffix(&name);
                if is_new {
                    builder = builder
                        .prefix(
                            Some(prefix.as_str().to_owned()),
                            namespace.as_str().to_owned(),
                        )
                        .unwrap();
                }
                builder = builder.attr(name, value);
            }
        }

        let element = builder.build();
        Self {
            inner: Some(element),
            nested: None,
        }
    }
}

impl FromEventsBuilder for ElementFromEvents {
    type Output = minidom::Element;

    fn feed(&mut self, ev: Event) -> Result<Option<Self::Output>, Error> {
        let inner = self
            .inner
            .as_mut()
            .expect("feed() called after it finished");
        if let Some(nested) = self.nested.as_mut() {
            match nested.feed(ev)? {
                Some(v) => {
                    inner.append_child(v);
                    self.nested = None;
                    return Ok(None);
                }
                None => return Ok(None),
            }
        }
        match ev {
            Event::XmlDeclaration(_, _) => Ok(None),
            Event::StartElement(_, qname, attrs) => {
                let nested = match Element::from_events(qname, attrs) {
                    Ok(v) => v,
                    Err(FromEventsError::Invalid(e)) => return Err(e),
                    Err(FromEventsError::Mismatch { .. }) => {
                        unreachable!("<Element as FromXml>::from_events should accept everything!")
                    }
                };
                self.nested = Some(Box::new(nested));
                Ok(None)
            }
            Event::Text(_, text) => {
                inner.append_text_node(text);
                Ok(None)
            }
            Event::EndElement(_) => Ok(Some(self.inner.take().unwrap())),
        }
    }
}

impl FromXml for Element {
    type Builder = ElementFromEvents;

    fn from_events(
        qname: rxml::QName,
        attrs: rxml::AttrMap,
    ) -> Result<Self::Builder, FromEventsError> {
        Ok(Self::Builder::new(qname, attrs))
    }
}

/// Helper struct to streamingly parse a struct which implements conversion
/// from [`minidom::Element`].
pub struct FromEventsViaElement<T> {
    inner: ElementFromEvents,
    // needed here because we need to keep the type `T` around until
    // `FromEventsBuilder` is done and it must always be the same type, so we
    // have to nail it down in the struct's type, and to do that we need to
    // bind it to a field. that's what PhantomData is for.
    _phantom: PhantomData<T>,
}

impl<E, T: TryFrom<minidom::Element, Error = E>> FromEventsViaElement<T>
where
    Error: From<E>,
{
    /// Create a new streaming parser for `T`.
    pub fn new(qname: rxml::QName, attrs: rxml::AttrMap) -> Result<Self, FromEventsError> {
        Ok(Self {
            _phantom: PhantomData,
            inner: Element::from_events(qname, attrs)?,
        })
    }
}

impl<E, T: TryFrom<minidom::Element, Error = E>> FromEventsBuilder for FromEventsViaElement<T>
where
    Error: From<E>,
{
    type Output = T;

    fn feed(&mut self, ev: Event) -> Result<Option<Self::Output>, Error> {
        match self.inner.feed(ev) {
            Ok(Some(v)) => Ok(Some(v.try_into()?)),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Helper struct to stream a struct which implements conversion
/// to [`minidom::Element`].
pub struct AsItemsViaElement<'x> {
    iter: EventToItem<IntoEvents>,
    lifetime_binding: PhantomData<Item<'x>>,
}

impl<'x> AsItemsViaElement<'x> {
    /// Create a new streaming parser for `T`.
    pub fn new<E, T>(value: T) -> Result<Self, crate::error::Error>
    where
        Error: From<E>,
        minidom::Element: TryFrom<T, Error = E>,
    {
        let element: minidom::Element = value.try_into()?;
        Ok(Self {
            iter: EventToItem::new(IntoEvents::new(element)),
            lifetime_binding: PhantomData,
        })
    }
}

impl<'x> Iterator for AsItemsViaElement<'x> {
    type Item = Result<Item<'x>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|x| x.map(Item::into_owned))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_element_is_equivalent() {
        let el: Element = "<foo xmlns='urn:a' a='b' c='d'><child a='x'/><child a='y'>some text</child><child xmlns='urn:b'><nested-child/></child></foo>".parse().unwrap();
        let transformed: Element = crate::transform(&el).unwrap();
        assert_eq!(el, transformed);
    }
}
