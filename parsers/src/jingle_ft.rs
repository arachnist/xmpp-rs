// Copyright (c) 2017 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::date::DateTime;
use crate::hashes::Hash;
use crate::jingle::{ContentId, Creator};
use crate::ns;
use minidom::{Element, Node};
use std::collections::BTreeMap;
use std::str::FromStr;
use xso::error::{Error, FromElementError};

generate_element!(
    /// Represents a range in a file.
    #[derive(Default)]
    Range, "range", JINGLE_FT,
    attributes: [
        /// The offset in bytes from the beginning of the file.
        offset: Default<u64> = "offset",

        /// The length in bytes of the range, or None to be the entire
        /// remaining of the file.
        length: Option<u64> = "length"
    ],
    children: [
        /// List of hashes for this range.
        hashes: Vec<Hash> = ("hash", HASHES) => Hash
    ]
);

impl Range {
    /// Creates a new range.
    pub fn new() -> Range {
        Default::default()
    }
}

type Lang = String;

generate_id!(
    /// Wrapper for a file description.
    Desc
);

/// Represents a file to be transferred.
#[derive(Debug, Clone, Default)]
pub struct File {
    /// The date of last modification of this file.
    pub date: Option<DateTime>,

    /// The MIME type of this file.
    pub media_type: Option<String>,

    /// The name of this file.
    pub name: Option<String>,

    /// The description of this file, possibly localised.
    pub descs: BTreeMap<Lang, Desc>,

    /// The size of this file, in bytes.
    pub size: Option<u64>,

    /// Used to request only a part of this file.
    pub range: Option<Range>,

    /// A list of hashes matching this entire file.
    pub hashes: Vec<Hash>,
}

impl File {
    /// Creates a new file descriptor.
    pub fn new() -> File {
        File::default()
    }

    /// Sets the date of last modification on this file.
    pub fn with_date(mut self, date: DateTime) -> File {
        self.date = Some(date);
        self
    }

    /// Sets the date of last modification on this file from an ISO-8601
    /// string.
    pub fn with_date_str(mut self, date: &str) -> Result<File, Error> {
        self.date = Some(DateTime::from_str(date).map_err(Error::text_parse_error)?);
        Ok(self)
    }

    /// Sets the MIME type of this file.
    pub fn with_media_type(mut self, media_type: String) -> File {
        self.media_type = Some(media_type);
        self
    }

    /// Sets the name of this file.
    pub fn with_name(mut self, name: String) -> File {
        self.name = Some(name);
        self
    }

    /// Sets a description for this file.
    pub fn add_desc(mut self, lang: &str, desc: Desc) -> File {
        self.descs.insert(Lang::from(lang), desc);
        self
    }

    /// Sets the file size of this file, in bytes.
    pub fn with_size(mut self, size: u64) -> File {
        self.size = Some(size);
        self
    }

    /// Request only a range of this file.
    pub fn with_range(mut self, range: Range) -> File {
        self.range = Some(range);
        self
    }

    /// Add a hash on this file.
    pub fn add_hash(mut self, hash: Hash) -> File {
        self.hashes.push(hash);
        self
    }
}

impl TryFrom<Element> for File {
    type Error = FromElementError;

    fn try_from(elem: Element) -> Result<File, FromElementError> {
        check_self!(elem, "file", JINGLE_FT);
        check_no_attributes!(elem, "file");

        let mut file = File {
            date: None,
            media_type: None,
            name: None,
            descs: BTreeMap::new(),
            size: None,
            range: None,
            hashes: vec![],
        };

        for child in elem.children() {
            if child.is("date", ns::JINGLE_FT) {
                if file.date.is_some() {
                    return Err(Error::Other("File must not have more than one date.").into());
                }
                file.date = Some(child.text().parse().map_err(Error::text_parse_error)?);
            } else if child.is("media-type", ns::JINGLE_FT) {
                if file.media_type.is_some() {
                    return Err(Error::Other("File must not have more than one media-type.").into());
                }
                file.media_type = Some(child.text());
            } else if child.is("name", ns::JINGLE_FT) {
                if file.name.is_some() {
                    return Err(Error::Other("File must not have more than one name.").into());
                }
                file.name = Some(child.text());
            } else if child.is("desc", ns::JINGLE_FT) {
                let lang = get_attr!(child, "xml:lang", Default);
                let desc = Desc(child.text());
                if file.descs.insert(lang, desc).is_some() {
                    return Err(
                        Error::Other("Desc element present twice for the same xml:lang.").into(),
                    );
                }
            } else if child.is("size", ns::JINGLE_FT) {
                if file.size.is_some() {
                    return Err(Error::Other("File must not have more than one size.").into());
                }
                file.size = Some(child.text().parse().map_err(Error::text_parse_error)?);
            } else if child.is("range", ns::JINGLE_FT) {
                if file.range.is_some() {
                    return Err(Error::Other("File must not have more than one range.").into());
                }
                file.range = Some(Range::try_from(child.clone())?);
            } else if child.is("hash", ns::HASHES) {
                file.hashes.push(Hash::try_from(child.clone())?);
            } else {
                return Err(Error::Other("Unknown element in JingleFT file.").into());
            }
        }

        Ok(file)
    }
}

impl From<File> for Element {
    fn from(file: File) -> Element {
        Element::builder("file", ns::JINGLE_FT)
            .append_all(
                file.date
                    .map(|date| Element::builder("date", ns::JINGLE_FT).append(date)),
            )
            .append_all(
                file.media_type.map(|media_type| {
                    Element::builder("media-type", ns::JINGLE_FT).append(media_type)
                }),
            )
            .append_all(
                file.name
                    .map(|name| Element::builder("name", ns::JINGLE_FT).append(name)),
            )
            .append_all(file.descs.into_iter().map(|(lang, desc)| {
                Element::builder("desc", ns::JINGLE_FT)
                    .attr("xml:lang", lang)
                    .append(desc.0)
            }))
            .append_all(
                file.size.map(|size| {
                    Element::builder("size", ns::JINGLE_FT).append(format!("{}", size))
                }),
            )
            .append_all(file.range)
            .append_all(file.hashes)
            .build()
    }
}

/// A wrapper element for a file.
#[derive(Debug, Clone)]
pub struct Description {
    /// The actual file descriptor.
    pub file: File,
}

impl TryFrom<Element> for Description {
    type Error = FromElementError;

    fn try_from(elem: Element) -> Result<Description, FromElementError> {
        check_self!(elem, "description", JINGLE_FT, "JingleFT description");
        check_no_attributes!(elem, "JingleFT description");
        let mut file = None;
        for child in elem.children() {
            if file.is_some() {
                return Err(Error::Other(
                    "JingleFT description element must have exactly one child.",
                )
                .into());
            }
            file = Some(File::try_from(child.clone())?);
        }
        if file.is_none() {
            return Err(
                Error::Other("JingleFT description element must have exactly one child.").into(),
            );
        }
        Ok(Description {
            file: file.unwrap(),
        })
    }
}

impl From<Description> for Element {
    fn from(description: Description) -> Element {
        Element::builder("description", ns::JINGLE_FT)
            .append(Node::Element(description.file.into()))
            .build()
    }
}

/// A checksum for checking that the file has been transferred correctly.
#[derive(Debug, Clone)]
pub struct Checksum {
    /// The identifier of the file transfer content.
    pub name: ContentId,

    /// The creator of this file transfer.
    pub creator: Creator,

    /// The file being checksummed.
    pub file: File,
}

impl TryFrom<Element> for Checksum {
    type Error = FromElementError;

    fn try_from(elem: Element) -> Result<Checksum, FromElementError> {
        check_self!(elem, "checksum", JINGLE_FT);
        check_no_unknown_attributes!(elem, "checksum", ["name", "creator"]);
        let mut file = None;
        for child in elem.children() {
            if file.is_some() {
                return Err(
                    Error::Other("JingleFT checksum element must have exactly one child.").into(),
                );
            }
            file = Some(match File::try_from(child.clone()) {
                Ok(v) => v,
                Err(FromElementError::Mismatch(_)) => {
                    return Err(Error::Other("Unexpected child element").into())
                }
                Err(other) => return Err(other),
            });
        }
        if file.is_none() {
            return Err(
                Error::Other("JingleFT checksum element must have exactly one child.").into(),
            );
        }
        Ok(Checksum {
            name: get_attr!(elem, "name", Required),
            creator: get_attr!(elem, "creator", Required),
            file: file.unwrap(),
        })
    }
}

impl From<Checksum> for Element {
    fn from(checksum: Checksum) -> Element {
        Element::builder("checksum", ns::JINGLE_FT)
            .attr("name", checksum.name)
            .attr("creator", checksum.creator)
            .append(Node::Element(checksum.file.into()))
            .build()
    }
}

generate_element!(
    /// A notice that the file transfer has been completed.
    Received, "received", JINGLE_FT,
    attributes: [
        /// The content identifier of this Jingle session.
        name: Required<ContentId> = "name",

        /// The creator of this file transfer.
        creator: Required<Creator> = "creator",
    ]
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Algo;
    use base64::{engine::general_purpose::STANDARD as Base64, Engine};

    // Apparently, i686 and AArch32/PowerPC seem to disagree here. So instead
    // of trying to figure this out now, we just ignore the test.
    #[cfg(target_pointer_width = "32")]
    #[test]
    #[ignore]
    fn test_size() {
        assert_size!(Range, 32);
        assert_size!(File, 104);
        assert_size!(Description, 104);
        assert_size!(Checksum, 128);
        assert_size!(Received, 16);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(Range, 48);
        assert_size!(File, 176);
        assert_size!(Description, 176);
        assert_size!(Checksum, 208);
        assert_size!(Received, 32);
    }

    #[test]
    fn test_description() {
        let elem: Element = r#"<description xmlns='urn:xmpp:jingle:apps:file-transfer:5'>
  <file>
    <media-type>text/plain</media-type>
    <name>test.txt</name>
    <date>2015-07-26T21:46:00+01:00</date>
    <size>6144</size>
    <hash xmlns='urn:xmpp:hashes:2'
          algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash>
  </file>
</description>
"#
        .parse()
        .unwrap();
        let desc = Description::try_from(elem).unwrap();
        assert_eq!(desc.file.media_type, Some(String::from("text/plain")));
        assert_eq!(desc.file.name, Some(String::from("test.txt")));
        assert_eq!(desc.file.descs, BTreeMap::new());
        assert_eq!(
            desc.file.date,
            DateTime::from_str("2015-07-26T21:46:00+01:00").ok()
        );
        assert_eq!(desc.file.size, Some(6144u64));
        assert_eq!(desc.file.range, None);
        assert_eq!(desc.file.hashes[0].algo, Algo::Sha_1);
        assert_eq!(
            desc.file.hashes[0].hash,
            Base64.decode("w0mcJylzCn+AfvuGdqkty2+KP48=").unwrap()
        );
    }

    #[test]
    fn test_request() {
        let elem: Element = r#"<description xmlns='urn:xmpp:jingle:apps:file-transfer:5'>
  <file>
    <hash xmlns='urn:xmpp:hashes:2'
          algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash>
  </file>
</description>
"#
        .parse()
        .unwrap();
        let desc = Description::try_from(elem).unwrap();
        assert_eq!(desc.file.media_type, None);
        assert_eq!(desc.file.name, None);
        assert_eq!(desc.file.descs, BTreeMap::new());
        assert_eq!(desc.file.date, None);
        assert_eq!(desc.file.size, None);
        assert_eq!(desc.file.range, None);
        assert_eq!(desc.file.hashes[0].algo, Algo::Sha_1);
        assert_eq!(
            desc.file.hashes[0].hash,
            Base64.decode("w0mcJylzCn+AfvuGdqkty2+KP48=").unwrap()
        );
    }

    #[test]
    fn test_descs() {
        let elem: Element = r#"<description xmlns='urn:xmpp:jingle:apps:file-transfer:5'>
  <file>
    <media-type>text/plain</media-type>
    <desc xml:lang='fr'>Fichier secret !</desc>
    <desc xml:lang='en'>Secret file!</desc>
    <hash xmlns='urn:xmpp:hashes:2'
          algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash>
  </file>
</description>
"#
        .parse()
        .unwrap();
        let desc = Description::try_from(elem).unwrap();
        assert_eq!(
            desc.file.descs.keys().cloned().collect::<Vec<_>>(),
            ["en", "fr"]
        );
        assert_eq!(desc.file.descs["en"], Desc(String::from("Secret file!")));
        assert_eq!(
            desc.file.descs["fr"],
            Desc(String::from("Fichier secret !"))
        );

        let elem: Element = r#"<description xmlns='urn:xmpp:jingle:apps:file-transfer:5'>
  <file>
    <media-type>text/plain</media-type>
    <desc xml:lang='fr'>Fichier secret !</desc>
    <desc xml:lang='fr'>Secret file!</desc>
    <hash xmlns='urn:xmpp:hashes:2'
          algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash>
  </file>
</description>
"#
        .parse()
        .unwrap();
        let error = Description::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Desc element present twice for the same xml:lang.");
    }

    #[test]
    fn test_received() {
        let elem: Element = "<received xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='initiator'/>".parse().unwrap();
        let received = Received::try_from(elem).unwrap();
        assert_eq!(received.name, ContentId(String::from("coucou")));
        assert_eq!(received.creator, Creator::Initiator);
        let elem2 = Element::from(received.clone());
        let received2 = Received::try_from(elem2).unwrap();
        assert_eq!(received2.name, ContentId(String::from("coucou")));
        assert_eq!(received2.creator, Creator::Initiator);

        let elem: Element = "<received xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='initiator'><coucou/></received>".parse().unwrap();
        let error = Received::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown child in received element.");

        let elem: Element =
            "<received xmlns='urn:xmpp:jingle:apps:file-transfer:5' creator='initiator'/>"
                .parse()
                .unwrap();
        let error = Received::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'name' missing.");

        let elem: Element = "<received xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='coucou'/>".parse().unwrap();
        let error = Received::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::TextParseError(string)) => string,
            _ => panic!(),
        };
        assert_eq!(
            message.to_string(),
            "Unknown value for 'creator' attribute."
        );
    }

    #[cfg(not(feature = "disable-validation"))]
    #[test]
    fn test_invalid_received() {
        let elem: Element = "<received xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='initiator' coucou=''/>".parse().unwrap();
        let error = Received::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown attribute in received element.");
    }

    #[test]
    fn test_checksum() {
        let elem: Element = "<checksum xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='initiator'><file><hash xmlns='urn:xmpp:hashes:2' algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash></file></checksum>".parse().unwrap();
        let hash = vec![
            195, 73, 156, 39, 41, 115, 10, 127, 128, 126, 251, 134, 118, 169, 45, 203, 111, 138,
            63, 143,
        ];
        let checksum = Checksum::try_from(elem).unwrap();
        assert_eq!(checksum.name, ContentId(String::from("coucou")));
        assert_eq!(checksum.creator, Creator::Initiator);
        assert_eq!(
            checksum.file.hashes,
            vec!(Hash {
                algo: Algo::Sha_1,
                hash: hash.clone()
            })
        );
        let elem2 = Element::from(checksum);
        let checksum2 = Checksum::try_from(elem2).unwrap();
        assert_eq!(checksum2.name, ContentId(String::from("coucou")));
        assert_eq!(checksum2.creator, Creator::Initiator);
        assert_eq!(
            checksum2.file.hashes,
            vec!(Hash {
                algo: Algo::Sha_1,
                hash: hash.clone()
            })
        );

        let elem: Element = "<checksum xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='initiator'><coucou/></checksum>".parse().unwrap();
        let error = Checksum::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            other => panic!("unexpected error: {:?}", other),
        };
        assert_eq!(message, "Unexpected child element");

        let elem: Element = "<checksum xmlns='urn:xmpp:jingle:apps:file-transfer:5' creator='initiator'><file><hash xmlns='urn:xmpp:hashes:2' algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash></file></checksum>".parse().unwrap();
        let error = Checksum::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Required attribute 'name' missing.");

        let elem: Element = "<checksum xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='coucou'><file><hash xmlns='urn:xmpp:hashes:2' algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash></file></checksum>".parse().unwrap();
        let error = Checksum::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::TextParseError(string)) => string,
            _ => panic!(),
        };
        assert_eq!(
            message.to_string(),
            "Unknown value for 'creator' attribute."
        );
    }

    #[cfg(not(feature = "disable-validation"))]
    #[test]
    fn test_invalid_checksum() {
        let elem: Element = "<checksum xmlns='urn:xmpp:jingle:apps:file-transfer:5' name='coucou' creator='initiator' coucou=''><file><hash xmlns='urn:xmpp:hashes:2' algo='sha-1'>w0mcJylzCn+AfvuGdqkty2+KP48=</hash></file></checksum>".parse().unwrap();
        let error = Checksum::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown attribute in checksum element.");
    }

    #[test]
    fn test_range() {
        let elem: Element = "<range xmlns='urn:xmpp:jingle:apps:file-transfer:5'/>"
            .parse()
            .unwrap();
        let range = Range::try_from(elem).unwrap();
        assert_eq!(range.offset, 0);
        assert_eq!(range.length, None);
        assert_eq!(range.hashes, vec!());

        let elem: Element = "<range xmlns='urn:xmpp:jingle:apps:file-transfer:5' offset='2048' length='1024'><hash xmlns='urn:xmpp:hashes:2' algo='sha-1'>kHp5RSzW/h7Gm1etSf90Mr5PC/k=</hash></range>".parse().unwrap();
        let hashes = vec![Hash {
            algo: Algo::Sha_1,
            hash: vec![
                144, 122, 121, 69, 44, 214, 254, 30, 198, 155, 87, 173, 73, 255, 116, 50, 190, 79,
                11, 249,
            ],
        }];
        let range = Range::try_from(elem).unwrap();
        assert_eq!(range.offset, 2048);
        assert_eq!(range.length, Some(1024));
        assert_eq!(range.hashes, hashes);
        let elem2 = Element::from(range);
        let range2 = Range::try_from(elem2).unwrap();
        assert_eq!(range2.offset, 2048);
        assert_eq!(range2.length, Some(1024));
        assert_eq!(range2.hashes, hashes);
    }

    #[cfg(not(feature = "disable-validation"))]
    #[test]
    fn test_invalid_range() {
        let elem: Element = "<range xmlns='urn:xmpp:jingle:apps:file-transfer:5' coucou=''/>"
            .parse()
            .unwrap();
        let error = Range::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Unknown attribute in range element.");
    }
}
