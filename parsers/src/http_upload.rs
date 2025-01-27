// Copyright (c) 2021 Maxime “pep” Buquet <pep@bouah.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{
    error::{Error, FromElementError, FromEventsError},
    exports::rxml,
    minidom_compat, AsXml, FromXml,
};

use crate::iq::{IqGetPayload, IqResultPayload};
use crate::ns;
use minidom::Element;

/// Requesting a slot
#[derive(FromXml, AsXml, Debug, Clone, PartialEq)]
#[xml(namespace = ns::HTTP_UPLOAD, name = "request")]
pub struct SlotRequest {
    /// The filename to be uploaded.
    #[xml(attribute)]
    pub filename: String,

    /// Size of the file to be uploaded.
    #[xml(attribute)]
    pub size: u64,

    /// Content-Type of the file.
    #[xml(attribute(name = "content-type"))]
    pub content_type: Option<String>,
}

impl IqGetPayload for SlotRequest {}

/// Slot header
#[derive(Debug, Clone, PartialEq)]
pub enum Header {
    /// Authorization header
    Authorization(String),

    /// Cookie header
    Cookie(String),

    /// Expires header
    Expires(String),
}

impl TryFrom<Element> for Header {
    type Error = FromElementError;
    fn try_from(elem: Element) -> Result<Header, FromElementError> {
        check_self!(elem, "header", HTTP_UPLOAD);
        check_no_children!(elem, "header");
        check_no_unknown_attributes!(elem, "header", ["name"]);
        let name: String = get_attr!(elem, "name", Required);
        let text = elem.text();

        Ok(match name.to_lowercase().as_str() {
            "authorization" => Header::Authorization(text),
            "cookie" => Header::Cookie(text),
            "expires" => Header::Expires(text),
            _ => {
                return Err(Error::Other(
                    "Header name must be either 'Authorization', 'Cookie', or 'Expires'.",
                )
                .into())
            }
        })
    }
}

impl FromXml for Header {
    type Builder = minidom_compat::FromEventsViaElement<Header>;

    fn from_events(
        qname: rxml::QName,
        attrs: rxml::AttrMap,
    ) -> Result<Self::Builder, FromEventsError> {
        if qname.0 != ns::HTTP_UPLOAD || qname.1 != "header" {
            return Err(FromEventsError::Mismatch { name: qname, attrs });
        }
        Self::Builder::new(qname, attrs)
    }
}

impl From<Header> for Element {
    fn from(elem: Header) -> Element {
        let (attr, val) = match elem {
            Header::Authorization(val) => ("Authorization", val),
            Header::Cookie(val) => ("Cookie", val),
            Header::Expires(val) => ("Expires", val),
        };

        Element::builder("header", ns::HTTP_UPLOAD)
            .attr("name", attr)
            .append(val)
            .build()
    }
}

impl AsXml for Header {
    type ItemIter<'x> = minidom_compat::AsItemsViaElement<'x>;

    fn as_xml_iter(&self) -> Result<Self::ItemIter<'_>, Error> {
        minidom_compat::AsItemsViaElement::new(self.clone())
    }
}

/// Put URL
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::HTTP_UPLOAD, name = "put")]
pub struct Put {
    /// URL
    #[xml(attribute)]
    pub url: String,

    /// Header list
    #[xml(child(n = ..))]
    pub headers: Vec<Header>,
}

/// Get URL
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::HTTP_UPLOAD, name = "get")]
pub struct Get {
    /// URL
    #[xml(attribute)]
    pub url: String,
}

/// Requesting a slot
#[derive(FromXml, AsXml, Debug, Clone, PartialEq)]
#[xml(namespace = ns::HTTP_UPLOAD, name = "slot")]
pub struct SlotResult {
    /// Put URL and headers
    #[xml(child)]
    pub put: Put,

    /// Get URL
    #[xml(child)]
    pub get: Get,
}

impl IqResultPayload for SlotResult {}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(SlotRequest, 32);
        assert_size!(Header, 16);
        assert_size!(Put, 24);
        assert_size!(Get, 12);
        assert_size!(SlotResult, 36);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(SlotRequest, 56);
        assert_size!(Header, 32);
        assert_size!(Put, 48);
        assert_size!(Get, 24);
        assert_size!(SlotResult, 72);
    }

    #[test]
    fn test_slot_request() {
        let elem: Element = "<request xmlns='urn:xmpp:http:upload:0'
            filename='très cool.jpg'
            size='23456'
            content-type='image/jpeg' />"
            .parse()
            .unwrap();
        let slot = SlotRequest::try_from(elem).unwrap();
        assert_eq!(slot.filename, String::from("très cool.jpg"));
        assert_eq!(slot.size, 23456);
        assert_eq!(slot.content_type, Some(String::from("image/jpeg")));
    }

    #[test]
    fn test_slot_result() {
        let elem: Element = "<slot xmlns='urn:xmpp:http:upload:0'>
            <put url='https://upload.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg'>
              <header name='Authorization'>Basic Base64String==</header>
              <header name='Cookie'>foo=bar; user=romeo</header>
            </put>
            <get url='https://download.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg' />
          </slot>"
            .parse()
            .unwrap();
        let slot = SlotResult::try_from(elem).unwrap();
        assert_eq!(slot.put.url, String::from("https://upload.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg"));
        assert_eq!(
            slot.put.headers[0],
            Header::Authorization(String::from("Basic Base64String=="))
        );
        assert_eq!(
            slot.put.headers[1],
            Header::Cookie(String::from("foo=bar; user=romeo"))
        );
        assert_eq!(slot.get.url, String::from("https://download.montague.tld/4a771ac1-f0b2-4a4a-9700-f2a26fa2bb67/tr%C3%A8s%20cool.jpg"));
    }

    #[test]
    fn test_result_no_header() {
        let elem: Element = "<slot xmlns='urn:xmpp:http:upload:0'>
            <put url='https://URL' />
            <get url='https://URL' />
          </slot>"
            .parse()
            .unwrap();
        let slot = SlotResult::try_from(elem).unwrap();
        assert_eq!(slot.put.url, String::from("https://URL"));
        assert_eq!(slot.put.headers.len(), 0);
        assert_eq!(slot.get.url, String::from("https://URL"));
    }

    #[test]
    fn test_result_bad_header() {
        let elem: Element = "<slot xmlns='urn:xmpp:http:upload:0'>
            <put url='https://URL'>
              <header name='EvilHeader'>EvilValue</header>
            </put>
            <get url='https://URL' />
          </slot>"
            .parse()
            .unwrap();
        SlotResult::try_from(elem).unwrap_err();
    }
}
