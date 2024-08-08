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

#[test]
fn parent_negative_duplicate_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<Parent>("<parent xmlns='urn:example:ns1'><attr foo='hello world!'/><attr foo='hello world!'/></parent>") {
        Err(::xso::error::FromElementError::Invalid(::xso::error::Error::Other(e))) if e.contains("must not have more than one") => (),
        other => panic!("unexpected result: {:?}", other),
    }
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

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, exhaustive)]
enum ExhaustiveNameSwitchedEnum {
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
fn exhaustive_name_switched_enum_negative_name_mismatch() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<ExhaustiveNameSwitchedEnum>("<x xmlns='urn:example:ns1'>hello</x>") {
        Err(xso::error::FromElementError::Invalid { .. }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn exhaustive_name_switched_enum_negative_namespace_mismatch() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<ExhaustiveNameSwitchedEnum>("<b xmlns='urn:example:ns2'>hello</b>") {
        Err(xso::error::FromElementError::Mismatch { .. }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn exhaustive_name_switched_enum_roundtrip_variant_1() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<ExhaustiveNameSwitchedEnum>("<a xmlns='urn:example:ns1' foo='hello'/>")
}

#[test]
fn exhaustive_name_switched_enum_roundtrip_variant_2() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<ExhaustiveNameSwitchedEnum>("<b xmlns='urn:example:ns1'>hello</b>")
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct Children {
    #[xml(child(n = ..))]
    foo: Vec<RequiredAttribute>,
}

#[test]
fn children_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<Children>(
        "<parent xmlns='urn:example:ns1'><attr foo='X'/><attr foo='Y'/><attr foo='Z'/></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct TextExtract {
    #[xml(extract(namespace = NS1, name = "child", fields(text)))]
    contents: String,
}

#[test]
fn text_extract_positive() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextExtract>(
        "<parent xmlns='urn:example:ns1'><child>hello world</child></parent>",
    ) {
        Ok(TextExtract { contents }) => {
            assert_eq!(contents, "hello world");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_negative_absent_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextExtract>("<parent xmlns='urn:example:ns1'/>") {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e)))
            if e.contains("Missing child field") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_negative_unexpected_attribute_in_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextExtract>("<parent xmlns='urn:example:ns1'><child foo='bar'/></parent>") {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e)))
            if e.contains("Unknown attribute") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_negative_unexpected_child_in_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextExtract>(
        "<parent xmlns='urn:example:ns1'><child><quak/></child></parent>",
    ) {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e)))
            if e.contains("Unknown child in extraction") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_negative_duplicate_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextExtract>(
        "<parent xmlns='urn:example:ns1'><child>hello world</child><child>more</child></parent>",
    ) {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e)))
            if e.contains("must not have more than one") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextExtract>(
        "<parent xmlns='urn:example:ns1'><child>hello world!</child></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct AttributeExtract {
    #[xml(extract(namespace = NS1, name = "child", fields(attribute = "foo")))]
    contents: String,
}

#[test]
fn attribute_extract_positive() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<AttributeExtract>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    ) {
        Ok(AttributeExtract { contents }) => {
            assert_eq!(contents, "hello world");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn attribute_extract_negative_absent_attribute() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<AttributeExtract>("<parent xmlns='urn:example:ns1'><child/></parent>") {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e)))
            if e.contains("Required attribute") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn attribute_extract_negative_unexpected_text_in_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<AttributeExtract>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'>fnord</child></parent>",
    ) {
        Err(xso::error::FromElementError::Invalid(xso::error::Error::Other(e)))
            if e.contains("Unexpected text") =>
        {
            ()
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn attribute_extract_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<AttributeExtract>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct OptionalAttributeExtract {
    #[xml(extract(namespace = NS1, name = "child", fields(attribute(name = "foo", default))))]
    contents: ::std::option::Option<String>,
}

#[test]
fn optional_attribute_extract_positive_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeExtract>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    ) {
        Ok(OptionalAttributeExtract {
            contents: Some(contents),
        }) => {
            assert_eq!(contents, "hello world");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_extract_positive_present_empty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeExtract>(
        "<parent xmlns='urn:example:ns1'><child foo=''/></parent>",
    ) {
        Ok(OptionalAttributeExtract {
            contents: Some(contents),
        }) => {
            assert_eq!(contents, "");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_extract_positive_absent() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeExtract>("<parent xmlns='urn:example:ns1'><child/></parent>")
    {
        Ok(OptionalAttributeExtract { contents: None }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_extract_roundtrip_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeExtract>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    )
}

#[test]
fn optional_attribute_extract_roundtrip_present_empty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeExtract>(
        "<parent xmlns='urn:example:ns1'><child foo=''/></parent>",
    )
}

#[test]
fn optional_attribute_extract_roundtrip_absent() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeExtract>("<parent xmlns='urn:example:ns1'><child/></parent>")
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct ChildExtract {
    #[xml(extract(namespace = NS1, name = "child", fields(child)))]
    contents: RequiredAttribute,
}

#[test]
fn child_extract_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<ChildExtract>(
        "<parent xmlns='urn:example:ns1'><child><attr foo='hello world!'/></child></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct NestedExtract {
    #[xml(extract(namespace = NS1, name = "child", fields(
        extract(namespace = NS1, name = "grandchild", fields(text))
    )))]
    contents: String,
}

#[test]
fn nested_extract_positive() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<NestedExtract>(
        "<parent xmlns='urn:example:ns1'><child><grandchild>hello world</grandchild></child></parent>",
    ) {
        Ok(NestedExtract { contents }) => {
            assert_eq!(contents, "hello world");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn nested_extract_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<NestedExtract>("<parent xmlns='urn:example:ns1'><child><grandchild>hello world</grandchild></child></parent>")
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct ExtractOmitNamespace {
    #[xml(extract(name = "child", fields(text)))]
    contents: String,
}

#[test]
fn extract_omit_namespace_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<ExtractOmitNamespace>(
        "<parent xmlns='urn:example:ns1'><child>hello world!</child></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct ExtractOmitName {
    #[xml(extract(namespace = NS1, fields(text)))]
    contents: String,
}

#[test]
fn extract_omit_name_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<ExtractOmitName>(
        "<parent xmlns='urn:example:ns1'><contents>hello world!</contents></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct ExtractOmitNameAndNamespace {
    #[xml(extract(fields(text)))]
    contents: String,
}

#[test]
fn extract_omit_name_and_namespace_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<ExtractOmitNameAndNamespace>(
        "<parent xmlns='urn:example:ns1'><contents>hello world!</contents></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct TextExtractVec {
    #[xml(extract(n = .., namespace = NS1, name = "child", fields(text(type_ = String))))]
    contents: Vec<String>,
}

#[test]
fn text_extract_vec_positive_nonempty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextExtractVec>(
        "<parent xmlns='urn:example:ns1'><child>hello</child><child>world</child></parent>",
    ) {
        Ok(TextExtractVec { contents }) => {
            assert_eq!(contents[0], "hello");
            assert_eq!(contents[1], "world");
            assert_eq!(contents.len(), 2);
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_vec_positive_empty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextExtractVec>("<parent xmlns='urn:example:ns1'/>") {
        Ok(TextExtractVec { contents }) => {
            assert_eq!(contents.len(), 0);
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_vec_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextExtractVec>(
        "<parent xmlns='urn:example:ns1'><child>hello</child><child>world</child></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct AttributeExtractVec {
    #[xml(extract(n = .., namespace = NS1, name = "child", fields(attribute(type_ = String, name = "attr"))))]
    contents: Vec<String>,
}

#[test]
fn text_extract_attribute_vec_positive_nonempty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<AttributeExtractVec>(
        "<parent xmlns='urn:example:ns1'><child attr='hello'/><child attr='world'/></parent>",
    ) {
        Ok(AttributeExtractVec { contents }) => {
            assert_eq!(contents[0], "hello");
            assert_eq!(contents[1], "world");
            assert_eq!(contents.len(), 2);
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_attribute_vec_positive_empty() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<AttributeExtractVec>("<parent xmlns='urn:example:ns1'/>") {
        Ok(AttributeExtractVec { contents }) => {
            assert_eq!(contents.len(), 0);
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_extract_attribute_vec_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<AttributeExtractVec>(
        "<parent xmlns='urn:example:ns1'><child attr='hello'/><child attr='world'/></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct TextOptionalExtract {
    #[xml(extract(namespace = NS1, name = "child", default, fields(text(type_ = String))))]
    contents: ::std::option::Option<String>,
}

#[test]
fn text_optional_extract_positive_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextOptionalExtract>(
        "<parent xmlns='urn:example:ns1'><child>hello world</child></parent>",
    ) {
        Ok(TextOptionalExtract {
            contents: Some(contents),
        }) => {
            assert_eq!(contents, "hello world");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_optional_extract_positive_absent_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<TextOptionalExtract>("<parent xmlns='urn:example:ns1'/>") {
        Ok(TextOptionalExtract { contents: None }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn text_optional_extract_roundtrip_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextOptionalExtract>(
        "<parent xmlns='urn:example:ns1'><child>hello world!</child></parent>",
    )
}

#[test]
fn text_optional_extract_roundtrip_absent() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TextOptionalExtract>("<parent xmlns='urn:example:ns1'/>")
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct OptionalAttributeOptionalExtract {
    #[xml(extract(namespace = NS1, name = "child", default, fields(attribute(name = "foo", default))))]
    contents: ::std::option::Option<String>,
}

#[test]
fn optional_attribute_optional_extract_positive_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeOptionalExtract>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    ) {
        Ok(OptionalAttributeOptionalExtract {
            contents: Some(contents),
        }) => {
            assert_eq!(contents, "hello world");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_optional_extract_positive_absent_attribute() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeOptionalExtract>(
        "<parent xmlns='urn:example:ns1'><child/></parent>",
    ) {
        Ok(OptionalAttributeOptionalExtract { contents: None }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_optional_extract_positive_absent_element() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeOptionalExtract>("<parent xmlns='urn:example:ns1'/>") {
        Ok(OptionalAttributeOptionalExtract { contents: None }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_optional_extract_roundtrip_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeOptionalExtract>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    )
}

#[test]
fn optional_attribute_optional_extract_roundtrip_absent_attribute() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeOptionalExtract>(
        "<parent xmlns='urn:example:ns1'><child/></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct OptionalAttributeOptionalExtractDoubleOption {
    #[xml(extract(namespace = NS1, name = "child", default, fields(attribute(name = "foo", type_ = ::std::option::Option<String>, default))))]
    contents: ::std::option::Option<::std::option::Option<String>>,
}

#[test]
fn optional_attribute_optional_extract_double_option_positive_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeOptionalExtractDoubleOption>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    ) {
        Ok(OptionalAttributeOptionalExtractDoubleOption {
            contents: Some(Some(contents)),
        }) => {
            assert_eq!(contents, "hello world");
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_optional_extract_double_option_positive_absent_attribute() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeOptionalExtractDoubleOption>(
        "<parent xmlns='urn:example:ns1'><child/></parent>",
    ) {
        Ok(OptionalAttributeOptionalExtractDoubleOption {
            contents: Some(None),
        }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_optional_extract_double_option_positive_absent_element() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    match parse_str::<OptionalAttributeOptionalExtractDoubleOption>(
        "<parent xmlns='urn:example:ns1'/>",
    ) {
        Ok(OptionalAttributeOptionalExtractDoubleOption { contents: None }) => (),
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn optional_attribute_optional_extract_double_option_roundtrip_present() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeOptionalExtractDoubleOption>(
        "<parent xmlns='urn:example:ns1'><child foo='hello world'/></parent>",
    )
}

#[test]
fn optional_attribute_optional_extract_double_option_roundtrip_absent_attribute() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeOptionalExtractDoubleOption>(
        "<parent xmlns='urn:example:ns1'><child/></parent>",
    )
}

#[test]
fn optional_attribute_optional_extract_double_option_roundtrip_absent_child() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<OptionalAttributeOptionalExtractDoubleOption>(
        "<parent xmlns='urn:example:ns1'/>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = NS1, name = "parent")]
struct ElementCatchall {
    #[xml(element(n = ..))]
    children: Vec<::minidom::Element>,
}

#[test]
fn element_catchall_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<ElementCatchall>(
        "<parent xmlns='urn:example:ns1'><child><deeper/></child><child xmlns='urn:example:ns2'/><more-children/><yet-another-child/><child/></parent>",
    )
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(transparent)]
struct TransparentStruct(RequiredAttribute);

#[test]
fn transparent_struct_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TransparentStruct>("<attr xmlns='urn:example:ns1' foo='bar'/>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(transparent)]
struct TransparentStructNamed {
    foo: RequiredAttribute,
}

#[test]
fn transparent_struct_named_roundtrip() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<TransparentStructNamed>("<attr xmlns='urn:example:ns1' foo='bar'/>");
}

#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml()]
enum DynamicEnum {
    #[xml(transparent)]
    A(RequiredAttribute),

    #[xml(namespace = NS2, name = "b")]
    B {
        #[xml(text)]
        contents: String,
    },
}

#[test]
fn dynamic_enum_roundtrip_a() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<DynamicEnum>("<attr xmlns='urn:example:ns1' foo='bar'/>");
}

#[test]
fn dynamic_enum_roundtrip_b() {
    #[allow(unused_imports)]
    use std::{
        option::Option::{None, Some},
        result::Result::{Err, Ok},
    };
    roundtrip_full::<DynamicEnum>("<b xmlns='urn:example:ns2'>hello world</b>");
}
