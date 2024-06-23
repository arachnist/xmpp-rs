// Copyright (c) 2024 Paul Fariello <xmpp-parsers@fariello.eu>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::message::MessagePayload;

generate_element!(
    /// Defines associated out of band url.
    Oob, "x", OOB,
    children: [
        /// The associated URL.
        url: Required<String> = ("url", OOB) => String,
        /// An optionnal description of the out of band data.
        desc: Option<String> = ("desc", OOB) => String,
    ]
);

impl MessagePayload for Oob {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Element;
    use xso::error::{Error, FromElementError};

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn test_size() {
        assert_size!(Oob, 24);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn test_size() {
        assert_size!(Oob, 48);
    }

    #[test]
    fn test_simple() {
        let elem: Element = "<x xmlns='jabber:x:oob'><url>http://example.org</url></x>"
            .parse()
            .unwrap();
        Oob::try_from(elem).unwrap();
    }

    #[test]
    fn test_with_desc() {
        let elem: Element =
            "<x xmlns='jabber:x:oob'><url>http://example.org</url><desc>Example website</desc></x>"
                .parse()
                .unwrap();
        Oob::try_from(elem).unwrap();
    }

    #[test]
    fn test_invalid_child() {
        let elem: Element = "<x xmlns='jabber:x:oob'></x>".parse().unwrap();
        let error = Oob::try_from(elem).unwrap_err();
        let message = match error {
            FromElementError::Invalid(Error::Other(string)) => string,
            _ => panic!(),
        };
        assert_eq!(message, "Missing child url in x element.");
    }
}
