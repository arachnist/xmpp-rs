#![deny(missing_docs)]

//! Provides a type for Jabber IDs.
//!
//! For usage, check the documentation on the `Jid` struct.

#[macro_use]
extern crate failure_derive;

use std::convert::Into;
use std::fmt;
use std::str::FromStr;

/// An error that signifies that a `Jid` cannot be parsed from a string.
#[derive(Debug, Clone, PartialEq, Eq, Fail)]
pub enum JidParseError {
    /// Happens when there is no domain, that is either the string is empty,
    /// starts with a /, or contains the @/ sequence.
    #[fail(display = "no domain found in this JID")]
    NoDomain,

    /// Happens when there is no resource, that is string contains no /.
    #[fail(display = "no resource found in this full JID")]
    NoResource,

    /// Happens when the node is empty, that is the string starts with a @.
    #[fail(display = "nodepart empty despite the presence of a @")]
    EmptyNode,

    /// Happens when the resource is empty, that is the string ends with a /.
    #[fail(display = "resource empty despite the presence of a /")]
    EmptyResource,
}

/// An enum representing a Jabber ID. It can be either a `FullJid` or a `BareJid`.
#[derive(Debug, Clone, PartialEq)]
pub enum Jid {
    /// Bare Jid
    Bare(BareJid),

    /// Full Jid
    Full(FullJid),
}

impl FromStr for Jid {
    type Err = JidParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (ns, ds, rs): StringJid = _from_str(s)?;
        Ok(match rs {
            Some(rs) => Jid::Full(FullJid {
                node: ns,
                domain: ds,
                resource: rs,
            }),
            None => Jid::Bare(BareJid {
                node: ns,
                domain: ds,
            }),
        })
    }
}

impl From<Jid> for String {
    fn from(jid: Jid) -> String {
        match jid {
            Jid::Bare(bare) => String::from(bare),
            Jid::Full(full) => String::from(full),
        }
    }
}

/// A struct representing a Full Jabber ID.
///
/// A Full Jabber ID is composed of 3 components, of which one is optional:
///
///  - A node/name, `node`, which is the optional part before the @.
///  - A domain, `domain`, which is the mandatory part after the @ but before the /.
///  - A resource, `resource`, which is the part after the /.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FullJid {
    /// The node part of the Jabber ID, if it exists, else None.
    pub node: Option<String>,
    /// The domain of the Jabber ID.
    pub domain: String,
    /// The resource of the Jabber ID.
    pub resource: String,
}

/// A struct representing a Bare Jabber ID.
///
/// A Bare Jabber ID is composed of 2 components, of which one is optional:
///
///  - A node/name, `node`, which is the optional part before the @.
///  - A domain, `domain`, which is the mandatory part after the @ but before the /.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BareJid {
    /// The node part of the Jabber ID, if it exists, else None.
    pub node: Option<String>,
    /// The domain of the Jabber ID.
    pub domain: String,
}

impl From<FullJid> for String {
    fn from(jid: FullJid) -> String {
        let mut string = String::new();
        if let Some(ref node) = jid.node {
            string.push_str(node);
            string.push('@');
        }
        string.push_str(&jid.domain);
        string.push('/');
        string.push_str(&jid.resource);
        string
    }
}

impl From<BareJid> for String {
    fn from(jid: BareJid) -> String {
        let mut string = String::new();
        if let Some(ref node) = jid.node {
            string.push_str(node);
            string.push('@');
        }
        string.push_str(&jid.domain);
        string
    }
}

impl Into<BareJid> for FullJid {
    fn into(self) -> BareJid {
        BareJid {
            node: self.node,
            domain: self.domain,
        }
    }
}

impl fmt::Debug for FullJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "FullJID({})", self)
    }
}

impl fmt::Debug for BareJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "BareJID({})", self)
    }
}

impl fmt::Display for FullJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(String::from(self.clone()).as_ref())
    }
}

impl fmt::Display for BareJid {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str(String::from(self.clone()).as_ref())
    }
}

enum ParserState {
    Node,
    Domain,
    Resource,
}

type StringJid = (Option<String>, String, Option<String>);
fn _from_str(s: &str) -> Result<StringJid, JidParseError> {
    // TODO: very naive, may need to do it differently
    let iter = s.chars();
    let mut buf = String::with_capacity(s.len());
    let mut state = ParserState::Node;
    let mut node = None;
    let mut domain = None;
    let mut resource = None;
    for c in iter {
        match state {
            ParserState::Node => {
                match c {
                    '@' => {
                        if buf == "" {
                            return Err(JidParseError::EmptyNode);
                        }
                        state = ParserState::Domain;
                        node = Some(buf.clone()); // TODO: performance tweaks, do not need to copy it
                        buf.clear();
                    }
                    '/' => {
                        if buf == "" {
                            return Err(JidParseError::NoDomain);
                        }
                        state = ParserState::Resource;
                        domain = Some(buf.clone()); // TODO: performance tweaks
                        buf.clear();
                    }
                    c => {
                        buf.push(c);
                    }
                }
            }
            ParserState::Domain => {
                match c {
                    '/' => {
                        if buf == "" {
                            return Err(JidParseError::NoDomain);
                        }
                        state = ParserState::Resource;
                        domain = Some(buf.clone()); // TODO: performance tweaks
                        buf.clear();
                    }
                    c => {
                        buf.push(c);
                    }
                }
            }
            ParserState::Resource => {
                buf.push(c);
            }
        }
    }
    if !buf.is_empty() {
        match state {
            ParserState::Node => {
                domain = Some(buf);
            }
            ParserState::Domain => {
                domain = Some(buf);
            }
            ParserState::Resource => {
                resource = Some(buf);
            }
        }
    } else if let ParserState::Resource = state {
        return Err(JidParseError::EmptyResource);
    }
    Ok((node, domain.ok_or(JidParseError::NoDomain)?, resource))
}

impl FromStr for FullJid {
    type Err = JidParseError;

    fn from_str(s: &str) -> Result<FullJid, JidParseError> {
        let (ns, ds, rs): StringJid = _from_str(s)?;
        Ok(FullJid {
            node: ns,
            domain: ds,
            resource: rs.ok_or(JidParseError::NoResource)?,
        })
    }
}

impl FullJid {
    /// Constructs a Full Jabber ID containing all three components.
    ///
    /// This is of the form `node`@`domain`/`resource`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.node, Some("node".to_owned()));
    /// assert_eq!(jid.domain, "domain".to_owned());
    /// assert_eq!(jid.resource, "resource".to_owned());
    /// ```
    pub fn new<NS, DS, RS>(node: NS, domain: DS, resource: RS) -> FullJid
    where
        NS: Into<String>,
        DS: Into<String>,
        RS: Into<String>,
    {
        FullJid {
            node: Some(node.into()),
            domain: domain.into(),
            resource: resource.into(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the node swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.node, Some("node".to_owned()));
    ///
    /// let new_jid = jid.with_node("new_node");
    ///
    /// assert_eq!(new_jid.node, Some("new_node".to_owned()));
    /// ```
    pub fn with_node<NS>(&self, node: NS) -> FullJid
    where
        NS: Into<String>,
    {
        FullJid {
            node: Some(node.into()),
            domain: self.domain.clone(),
            resource: self.resource.clone(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the domain swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.domain, "domain".to_owned());
    ///
    /// let new_jid = jid.with_domain("new_domain");
    ///
    /// assert_eq!(new_jid.domain, "new_domain");
    /// ```
    pub fn with_domain<DS>(&self, domain: DS) -> FullJid
    where
        DS: Into<String>,
    {
        FullJid {
            node: self.node.clone(),
            domain: domain.into(),
            resource: self.resource.clone(),
        }
    }

    /// Constructs a Full Jabber ID from a Bare Jabber ID, specifying a `resource`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::FullJid;
    ///
    /// let jid = FullJid::new("node", "domain", "resource");
    ///
    /// assert_eq!(jid.resource, "resource".to_owned());
    ///
    /// let new_jid = jid.with_resource("new_resource");
    ///
    /// assert_eq!(new_jid.resource, "new_resource");
    /// ```
    pub fn with_resource<RS>(&self, resource: RS) -> FullJid
    where
        RS: Into<String>,
    {
        FullJid {
            node: self.node.clone(),
            domain: self.domain.clone(),
            resource: resource.into(),
        }
    }
}

impl FromStr for BareJid {
    type Err = JidParseError;

    fn from_str(s: &str) -> Result<BareJid, JidParseError> {
        let (ns, ds, _rs): StringJid = _from_str(s)?;
        Ok(BareJid {
            node: ns,
            domain: ds,
        })
    }
}

impl BareJid {
    /// Constructs a Bare Jabber ID, containing two components.
    ///
    /// This is of the form `node`@`domain`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::new("node", "domain");
    ///
    /// assert_eq!(jid.node, Some("node".to_owned()));
    /// assert_eq!(jid.domain, "domain".to_owned());
    /// ```
    pub fn new<NS, DS>(node: NS, domain: DS) -> BareJid
    where
        NS: Into<String>,
        DS: Into<String>,
    {
        BareJid {
            node: Some(node.into()),
            domain: domain.into(),
        }
    }

    /// Constructs a Bare Jabber ID containing only a `domain`.
    ///
    /// This is of the form `domain`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::domain("domain");
    ///
    /// assert_eq!(jid.node, None);
    /// assert_eq!(jid.domain, "domain".to_owned());
    /// ```
    pub fn domain<DS>(domain: DS) -> BareJid
    where
        DS: Into<String>,
    {
        BareJid {
            node: None,
            domain: domain.into(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the node swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::domain("domain");
    ///
    /// assert_eq!(jid.node, None);
    ///
    /// let new_jid = jid.with_node("node");
    ///
    /// assert_eq!(new_jid.node, Some("node".to_owned()));
    /// ```
    pub fn with_node<NS>(&self, node: NS) -> BareJid
    where
        NS: Into<String>,
    {
        BareJid {
            node: Some(node.into()),
            domain: self.domain.clone(),
        }
    }

    /// Constructs a new Jabber ID from an existing one, with the domain swapped out with a new one.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let jid = BareJid::domain("domain");
    ///
    /// assert_eq!(jid.domain, "domain");
    ///
    /// let new_jid = jid.with_domain("new_domain");
    ///
    /// assert_eq!(new_jid.domain, "new_domain");
    /// ```
    pub fn with_domain<DS>(&self, domain: DS) -> BareJid
    where
        DS: Into<String>,
    {
        BareJid {
            node: self.node.clone(),
            domain: domain.into(),
        }
    }

    /// Constructs a Full Jabber ID from a Bare Jabber ID, specifying a `resource`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jid::BareJid;
    ///
    /// let bare = BareJid::new("node", "domain");
    /// let full = bare.with_resource("resource");
    ///
    /// assert_eq!(full.node, Some("node".to_owned()));
    /// assert_eq!(full.domain, "domain".to_owned());
    /// assert_eq!(full.resource, "resource".to_owned());
    /// ```
    pub fn with_resource<RS>(self, resource: RS) -> FullJid
    where
        RS: Into<String>,
    {
        FullJid {
            node: self.node,
            domain: self.domain,
            resource: resource.into(),
        }
    }
}

#[cfg(feature = "minidom")]
use minidom::{ElementEmitter, IntoAttributeValue, IntoElements};

#[cfg(feature = "minidom")]
impl IntoAttributeValue for Jid {
    fn into_attribute_value(self) -> Option<String> {
        Some(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl IntoElements for Jid {
    fn into_elements(self, emitter: &mut ElementEmitter) {
        emitter.append_text_node(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for FullJid {
    fn into_attribute_value(self) -> Option<String> {
        Some(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl IntoElements for FullJid {
    fn into_elements(self, emitter: &mut ElementEmitter) {
        emitter.append_text_node(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl IntoAttributeValue for BareJid {
    fn into_attribute_value(self) -> Option<String> {
        Some(String::from(self))
    }
}

#[cfg(feature = "minidom")]
impl IntoElements for BareJid {
    fn into_elements(self, emitter: &mut ElementEmitter) {
        emitter.append_text_node(String::from(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn can_parse_full_jids() {
        assert_eq!(
            FullJid::from_str("a@b.c/d"),
            Ok(FullJid::new("a", "b.c", "d"))
        );
        assert_eq!(
            FullJid::from_str("b.c/d"),
            Ok(FullJid {
                node: None,
                domain: "b.c".to_owned(),
                resource: "d".to_owned(),
            })
        );

        assert_eq!(FullJid::from_str("a@b.c"), Err(JidParseError::NoResource));
        assert_eq!(FullJid::from_str("b.c"), Err(JidParseError::NoResource));
    }

    #[test]
    fn can_parse_bare_jids() {
        assert_eq!(BareJid::from_str("a@b.c/d"), Ok(BareJid::new("a", "b.c")));
        assert_eq!(
            BareJid::from_str("b.c/d"),
            Ok(BareJid {
                node: None,
                domain: "b.c".to_owned(),
            })
        );

        assert_eq!(BareJid::from_str("a@b.c"), Ok(BareJid::new("a", "b.c")));
        assert_eq!(
            BareJid::from_str("b.c"),
            Ok(BareJid {
                node: None,
                domain: "b.c".to_owned(),
            })
        );
    }

    #[test]
    fn can_parse_jids() {
        let full = FullJid::from_str("a@b.c/d").unwrap();
        let bare = BareJid::from_str("e@f.g").unwrap();

        assert_eq!(Jid::from_str("a@b.c/d"), Ok(Jid::Full(full)));
        assert_eq!(Jid::from_str("e@f.g"), Ok(Jid::Bare(bare)));
    }

    #[test]
    fn full_to_bare_jid() {
        let bare: BareJid = FullJid::new("a", "b.c", "d").into();
        assert_eq!(bare, BareJid::new("a", "b.c"));
    }

    #[test]
    fn bare_to_full_jid() {
        assert_eq!(
            BareJid::new("a", "b.c").with_resource("d"),
            FullJid::new("a", "b.c", "d")
        );
    }

    #[test]
    fn serialise() {
        assert_eq!(
            String::from(FullJid::new("a", "b", "c")),
            String::from("a@b/c")
        );
        assert_eq!(String::from(BareJid::new("a", "b")), String::from("a@b"));
    }

    #[test]
    fn invalid_jids() {
        assert_eq!(BareJid::from_str(""), Err(JidParseError::NoDomain));
        assert_eq!(BareJid::from_str("/c"), Err(JidParseError::NoDomain));
        assert_eq!(BareJid::from_str("a@/c"), Err(JidParseError::NoDomain));
        assert_eq!(BareJid::from_str("@b"), Err(JidParseError::EmptyNode));
        assert_eq!(BareJid::from_str("b/"), Err(JidParseError::EmptyResource));

        assert_eq!(FullJid::from_str(""), Err(JidParseError::NoDomain));
        assert_eq!(FullJid::from_str("/c"), Err(JidParseError::NoDomain));
        assert_eq!(FullJid::from_str("a@/c"), Err(JidParseError::NoDomain));
        assert_eq!(FullJid::from_str("@b"), Err(JidParseError::EmptyNode));
        assert_eq!(FullJid::from_str("b/"), Err(JidParseError::EmptyResource));
        assert_eq!(FullJid::from_str("a@b"), Err(JidParseError::NoResource));
    }

    #[cfg(feature = "minidom")]
    #[test]
    fn minidom() {
        let elem: minidom::Element = "<message from='a@b/c'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::Full(FullJid::new("a", "b", "c")));

        let elem: minidom::Element = "<message from='a@b'/>".parse().unwrap();
        let to: Jid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, Jid::Bare(BareJid::new("a", "b")));

        let elem: minidom::Element = "<message from='a@b/c'/>".parse().unwrap();
        let to: FullJid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, FullJid::new("a", "b", "c"));

        let elem: minidom::Element = "<message from='a@b'/>".parse().unwrap();
        let to: BareJid = elem.attr("from").unwrap().parse().unwrap();
        assert_eq!(to, BareJid::new("a", "b"));
    }

    #[cfg(feature = "minidom")]
    #[test]
    fn minidom_into_attr() {
        let full = FullJid::new("a", "b", "c");
        let elem = minidom::Element::builder("message")
            .ns("jabber:client")
            .attr("from", full.clone())
            .build();
        assert_eq!(elem.attr("from"), Some(String::from(full).as_ref()));

        let bare = BareJid::new("a", "b");
        let elem = minidom::Element::builder("message")
            .ns("jabber:client")
            .attr("from", bare.clone())
            .build();
        assert_eq!(elem.attr("from"), Some(String::from(bare.clone()).as_ref()));

        let jid = Jid::Bare(bare.clone());
        let _elem = minidom::Element::builder("message")
            .ns("jabber:client")
            .attr("from", jid)
            .build();
        assert_eq!(elem.attr("from"), Some(String::from(bare).as_ref()));
    }
}
