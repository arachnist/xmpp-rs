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
    use xso::{error::FromElementError, transform, try_from_element, AsXml, FromXml};

    pub(super) fn roundtrip_full<T: AsXml + FromXml + PartialEq + std::fmt::Debug + Clone>(
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

use xso::{AsXml, FromXml};

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
static NS2: &str = "urn:example:ns2";

static FOO_NAME: &::xso::exports::rxml::strings::NcNameStr = {
    #[allow(unsafe_code)]
    unsafe {
        ::xso::exports::rxml::strings::NcNameStr::from_str_unchecked("foo")
    }
};

static BAR_NAME: &::xso::exports::rxml::strings::NcNameStr = {
    #[allow(unsafe_code)]
    unsafe {
        ::xso::exports::rxml::strings::NcNameStr::from_str_unchecked("bar")
    }
};

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
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

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = BAR_NAME)]
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

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
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

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "attr")]
struct RequiredAttribute {
    #[xml(attribute)]
    foo: String,
}

#[test]
fn required_attribute_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<RequiredAttribute>("<attr xmlns='urn:example:ns1' foo='bar'/>");
}

#[test]
fn required_attribute_positive() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    let data = parse_str::<RequiredAttribute>("<attr xmlns='urn:example:ns1' foo='bar'/>").unwrap();
    assert_eq!(data.foo, "bar");
}

#[test]
fn required_attribute_missing() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<RequiredAttribute>("<attr xmlns='urn:example:ns1'/>") {
        Err(::xso::error::FromElementError::Invalid(::xso::error::Error::Other(e)))
            if e.contains("Required attribute field") && e.contains("missing") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "attr")]
struct RenamedAttribute {
    #[xml(attribute = "a1")]
    foo: String,
    #[xml(attribute = BAR_NAME)]
    bar: String,
}

#[test]
fn renamed_attribute_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<RenamedAttribute>("<attr xmlns='urn:example:ns1' a1='bar' bar='baz'/>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "attr")]
struct NamespacedAttribute {
    #[xml(attribute(namespace = "urn:example:ns1", name = FOO_NAME))]
    foo_1: String,
    #[xml(attribute(namespace = NS2, name = "foo"))]
    foo_2: String,
    #[xml(attribute(namespace = NS1, name = BAR_NAME))]
    bar_1: String,
    #[xml(attribute(namespace = "urn:example:ns2", name = "bar"))]
    bar_2: String,
}

#[test]
fn namespaced_attribute_roundtrip_a() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<NamespacedAttribute>(
        "<attr xmlns='urn:example:ns1'
          xmlns:tns0='urn:example:ns1' tns0:foo='a1' tns0:bar='a3'
          xmlns:tns1='urn:example:ns2' tns1:foo='a2' tns1:bar='a4'/>",
    );
}

#[test]
fn namespaced_attribute_roundtrip_b() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<NamespacedAttribute>(
        "<tns0:attr
          xmlns:tns0='urn:example:ns1' tns0:foo='a1' tns0:bar='a3'
          xmlns:tns1='urn:example:ns2' tns1:foo='a2' tns1:bar='a4'/>",
    );
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "attr")]
struct PrefixedAttribute {
    #[xml(attribute = "xml:lang")]
    lang: String,
}

#[test]
fn prefixed_attribute_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<PrefixedAttribute>("<attr xmlns='urn:example:ns1' xml:lang='foo'/>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "attr")]
struct RequiredNonStringAttribute {
    #[xml(attribute)]
    foo: i32,
}

#[test]
fn required_non_string_attribute_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<RequiredNonStringAttribute>("<attr xmlns='urn:example:ns1' foo='-16'/>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "attr")]
struct DefaultAttribute {
    #[xml(attribute(default))]
    foo: std::option::Option<String>,

    #[xml(attribute(default))]
    bar: std::option::Option<u16>,
}

#[test]
fn default_attribute_roundtrip_aa() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<DefaultAttribute>("<attr xmlns='urn:example:ns1'/>");
}

#[test]
fn default_attribute_roundtrip_pa() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<DefaultAttribute>("<attr xmlns='urn:example:ns1' foo='xyz'/>");
}

#[test]
fn default_attribute_roundtrip_ap() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<DefaultAttribute>("<attr xmlns='urn:example:ns1' bar='16'/>");
}

#[test]
fn default_attribute_roundtrip_pp() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<DefaultAttribute>("<attr xmlns='urn:example:ns1' foo='xyz' bar='16'/>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "text")]
struct TextString {
    #[xml(text)]
    text: String,
}

#[test]
fn text_string_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextString>("<text xmlns='urn:example:ns1'>hello world!</text>");
}

#[test]
fn text_string_positive_preserves_whitespace() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    let el = parse_str::<TextString>("<text xmlns='urn:example:ns1'> \t\n</text>").unwrap();
    assert_eq!(el.text, " \t\n");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "text")]
struct TextNonString {
    #[xml(text)]
    text: u32,
}

#[test]
fn text_non_string_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextNonString>("<text xmlns='urn:example:ns1'>123456</text>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "elem")]
struct IgnoresWhitespaceWithoutTextConsumer;

#[test]
fn ignores_whitespace_without_text_consumer_positive() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    let _ = parse_str::<IgnoresWhitespaceWithoutTextConsumer>(
        "<elem xmlns='urn:example:ns1'> \t\r\n</elem>",
    )
    .unwrap();
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "elem")]
struct FailsTextWithoutTextConsumer;

#[test]
fn fails_text_without_text_consumer_positive() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<FailsTextWithoutTextConsumer>("<elem xmlns='urn:example:ns1'>  quak  </elem>")
    {
        Err(::xso::error::FromElementError::Invalid(::xso::error::Error::Other(e)))
            if e.contains("Unexpected text") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "text")]
struct TextWithCodec {
    #[xml(text(codec = xso::text::EmptyAsNone))]
    text: std::option::Option<String>,
}

#[test]
fn text_with_codec_roundtrip_empty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextWithCodec>("<text xmlns='urn:example:ns1'/>");
}

#[test]
fn text_with_codec_roundtrip_non_empty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextWithCodec>("<text xmlns='urn:example:ns1'>hello</text>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct Parent {
    #[xml(child)]
    child: RequiredAttribute,
}

#[test]
fn parent_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<Parent>("<parent xmlns='urn:example:ns1'><attr foo='hello world!'/></parent>")
}

#[test]
fn parent_positive() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    let v =
        parse_str::<Parent>("<parent xmlns='urn:example:ns1'><attr foo='hello world!'/></parent>")
            .unwrap();
    assert_eq!(v.child.foo, "hello world!");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct OptionalChild {
    #[xml(child(default))]
    child: std::option::Option<RequiredAttribute>,
}

#[test]
fn optional_child_roundtrip_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalChild>(
        "<parent xmlns='urn:example:ns1'><attr foo='hello world!'/></parent>",
    )
}

#[test]
fn optional_child_roundtrip_absent() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalChild>("<parent xmlns='urn:example:ns1'/>")
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "elem")]
struct BoxedChild {
    #[xml(child(default))]
    child: std::option::Option<Box<BoxedChild>>,
}

#[test]
fn boxed_child_roundtrip_absent() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<BoxedChild>("<elem xmlns='urn:example:ns1'/>")
}

#[test]
fn boxed_child_roundtrip_nested_1() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<BoxedChild>("<elem xmlns='urn:example:ns1'><elem/></elem>")
}

#[test]
fn boxed_child_roundtrip_nested_2() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<BoxedChild>("<elem xmlns='urn:example:ns1'><elem><elem/></elem></elem>")
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "elem", builder = RenamedBuilder, iterator = RenamedIter)]
struct RenamedTypes;

#[test]
fn renamed_types_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<RenamedTypes>("<elem xmlns='urn:example:ns1'/>")
}

#[test]
#[allow(unused_comparisons)]
fn renamed_types_get_renamed() {
    // these merely serve as a test that the types are declared with the names
    // given in the attributes.
    assert!(std::mem::size_of::<RenamedBuilder>() >= 0);
    assert!(std::mem::size_of::<RenamedIter>() >= 0);
}

// What is this, you may wonder?
// This is a test that any generated type names won't trigger
// the `non_camel_case_types` lint.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "elem")]
struct LintTest_;

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1)]
enum NameSwitchedEnum {
    #[xml(name = "a")]
    Variant1 {
        #[xml(attribute)]
        foo: String,
    },
    #[xml(name = "b")]
    Variant2 {
        #[xml(text)]
        foo: String,
    },
}

#[test]
fn name_switched_enum_positive_variant_1() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<NameSwitchedEnum>("<a xmlns='urn:example:ns1' foo='hello'/>") {
        Ok(NameSwitchedEnum::Variant1 { foo }) => {
            assert_eq!(foo, "hello");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn name_switched_enum_positive_variant_2() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<NameSwitchedEnum>("<b xmlns='urn:example:ns1'>hello</b>") {
        Ok(NameSwitchedEnum::Variant2 { foo }) => {
            assert_eq!(foo, "hello");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn name_switched_enum_negative_name_mismatch() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<NameSwitchedEnum>("<x xmlns='urn:example:ns1'>hello</x>") {
        Err(xso::error::FromElementError::Mismatch { .. }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn name_switched_enum_negative_namespace_mismatch() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<NameSwitchedEnum>("<b xmlns='urn:example:ns2'>hello</b>") {
        Err(xso::error::FromElementError::Mismatch { .. }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn name_switched_enum_roundtrip_variant_1() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<NameSwitchedEnum>("<a xmlns='urn:example:ns1' foo='hello'/>")
}

#[test]
fn name_switched_enum_roundtrip_variant_2() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<NameSwitchedEnum>("<b xmlns='urn:example:ns1'>hello</b>")
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, builder = RenamedEnumBuilder, iterator = RenamedEnumIter)]
enum RenamedEnumTypes {
    #[xml(name = "elem")]
    A,
}

#[test]
fn renamed_enum_types_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<RenamedEnumTypes>("<elem xmlns='urn:example:ns1'/>")
}

#[test]
#[allow(unused_comparisons)]
fn renamed_enum_types_get_renamed() {
    // these merely serve as a test that the types are declared with the names
    // given in the attributes.
    assert!(std::mem::size_of::<RenamedEnumBuilder>() >= 0);
    assert!(std::mem::size_of::<RenamedEnumIter>() >= 0);
}
