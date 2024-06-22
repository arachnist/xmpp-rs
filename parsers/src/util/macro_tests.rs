// Copyright (c) 2024 Jonas Schäfer <jonas@zombofant.net>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(
    non_camel_case_types,
    non_snake_case,
    unsafe_code,
    unused_variables,
    unused_mut,
    dead_code
)]

mod helpers {
    // we isolate the helpers into a module, because we do not want to have
    // them in scope below.
    // this is to ensure that the macros do not have hidden dependencies on
    // any specific names being imported.
    use minidom::Element;
    use xso::{error::FromElementError, transform, try_from_element, FromXml, IntoXml};

    pub(super) fn roundtrip_full<T: IntoXml + FromXml + PartialEq + std::fmt::Debug + Clone>(
        s: &str,
    ) {
        let initial: Element = s.parse().unwrap();
        let structural: T = match try_from_element(initial.clone()) {
            Ok(v) => v,
            Err(e) => panic!("failed to parse from {:?}: {}", s, e),
        };
        let recovered =
            transform(structural.clone()).expect("roundtrip did not produce an element");
        assert_eq!(initial, recovered);
        let structural2: T = match try_from_element(recovered) {
            Ok(v) => v,
            Err(e) => panic!("failed to parse from serialisation of {:?}: {}", s, e),
        };
        assert_eq!(structural, structural2);
    }

    pub(super) fn parse_str<T: FromXml>(s: &str) -> Result<T, FromElementError> {
        let initial: Element = s.parse().unwrap();
        try_from_element(initial)
    }
}

use self::helpers::{parse_str, roundtrip_full};

use xso::{FromXml, IntoXml};

// these are adverserial local names in order to trigger any issues with
// unqualified names in the macro expansions.
#[allow(dead_code, non_snake_case)]
fn Err() {}
#[allow(dead_code, non_snake_case)]
fn Ok() {}
#[allow(dead_code, non_snake_case)]
fn Some() {}
#[allow(dead_code, non_snake_case)]
fn None() {}
#[allow(dead_code)]
type Option = ((),);
#[allow(dead_code)]
type Result = ((),);

static NS1: &str = "urn:example:ns1";

#[derive(FromXml, IntoXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "foo")]
struct Empty;

#[test]
fn empty_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<Empty>("<foo xmlns='urn:example:ns1'/>");
}

#[test]
fn empty_name_mismatch() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<Empty>("<bar xmlns='urn:example:ns1'/>") {
        Err(xso::error::FromElementError::Mismatch(..)) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn empty_namespace_mismatch() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<Empty>("<foo xmlns='urn:example:ns2'/>") {
        Err(xso::error::FromElementError::Mismatch(..)) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn empty_unexpected_attribute() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<Empty>("<foo xmlns='urn:example:ns1' fnord='bar'/>") {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e))) => {
            assert_eq!(e, "Unknown attribute in Empty element.");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn empty_unexpected_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<Empty>("<foo xmlns='urn:example:ns1'><coucou/></foo>") {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e))) => {
            assert_eq!(e, "Unknown child in Empty element.");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn empty_qname_check_has_precedence_over_attr_check() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<Empty>("<bar xmlns='urn:example:ns1' fnord='bar'/>") {
        Err(xso::error::FromElementError::Mismatch(..)) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

static SOME_NAME: &::xso::exports::rxml::strings::NcNameStr = {
    #[allow(unsafe_code)]
    unsafe {
        ::xso::exports::rxml::strings::NcNameStr::from_str_unchecked("bar")
    }
};

#[derive(FromXml, IntoXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = SOME_NAME)]
struct NamePath;

#[test]
fn name_path_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<NamePath>("<bar xmlns='urn:example:ns1'/>");
}

#[derive(FromXml, IntoXml, PartialEq, Debug, Clone)]
#[xml(namespace = "urn:example:ns2", name = "baz")]
struct NamespaceLit;

#[test]
fn namespace_lit_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<NamespaceLit>("<baz xmlns='urn:example:ns2'/>");
}
