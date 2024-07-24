// Copyright (c) 2017-2018 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

macro_rules! get_attr {
    ($elem:ident, $attr:tt, $type:tt) => {
        get_attr!(
            $elem,
            $attr,
            $type,
            value,
            value.parse().map_err(xso::error::Error::text_parse_error)?
        )
    };
    ($elem:ident, $attr:tt, OptionEmpty, $value:ident, $func:expr) => {
        match $elem.attr($attr) {
            Some("") => None,
            Some($value) => Some($func),
            None => None,
        }
    };
    ($elem:ident, $attr:tt, Option, $value:ident, $func:expr) => {
        match $elem.attr($attr) {
            Some($value) => Some($func),
            None => None,
        }
    };
    ($elem:ident, $attr:tt, Required, $value:ident, $func:expr) => {
        match $elem.attr($attr) {
            Some($value) => $func,
            None => {
                return Err(xso::error::Error::Other(
                    concat!("Required attribute '", $attr, "' missing.").into(),
                )
                .into());
            }
        }
    };
    ($elem:ident, $attr:tt, RequiredNonEmpty, $value:ident, $func:expr) => {
        match $elem.attr($attr) {
            Some("") => {
                return Err(xso::error::Error::Other(
                    concat!("Required attribute '", $attr, "' must not be empty.").into(),
                )
                .into());
            }
            Some($value) => $func,
            None => {
                return Err(xso::error::Error::Other(
                    concat!("Required attribute '", $attr, "' missing.").into(),
                )
                .into());
            }
        }
    };
    ($elem:ident, $attr:tt, Default, $value:ident, $func:expr) => {
        match $elem.attr($attr) {
            Some($value) => $func,
            None => ::std::default::Default::default(),
        }
    };
}

macro_rules! generate_attribute {
    ($(#[$meta:meta])* $elem:ident, $name:tt, {$($(#[$a_meta:meta])* $a:ident => $b:tt),+$(,)?}) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub enum $elem {
            $(
                $(#[$a_meta])*
                $a
            ),+
        }
        impl ::std::str::FromStr for $elem {
            type Err = xso::error::Error;
            fn from_str(s: &str) -> Result<$elem, xso::error::Error> {
                Ok(match s {
                    $($b => $elem::$a),+,
                    _ => return Err(xso::error::Error::Other(concat!("Unknown value for '", $name, "' attribute.")).into()),
                })
            }
        }
        impl ::xso::FromXmlText for $elem {
            fn from_xml_text(s: String) -> Result<$elem, xso::error::Error> {
                s.parse().map_err(xso::error::Error::text_parse_error)
            }
        }
        impl std::fmt::Display for $elem {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                write!(fmt, "{}", match self {
                    $($elem::$a => $b),+
                })
            }
        }
        impl ::xso::AsXmlText for $elem {
            fn as_xml_text(&self) -> Result<::std::borrow::Cow<'_, str>, xso::error::Error> {
                match self {
                    $(
                        $elem::$a => Ok(::std::borrow::Cow::Borrowed($b))
                    ),+
                }
            }
        }
        impl ::minidom::IntoAttributeValue for $elem {
            fn into_attribute_value(self) -> Option<String> {
                Some(String::from(match self {
                    $($elem::$a => $b),+
                }))
            }
        }
    );
    ($(#[$meta:meta])* $elem:ident, $name:tt, {$($(#[$a_meta:meta])* $a:ident => $b:tt),+$(,)?}, Default = $default:ident) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub enum $elem {
            $(
                $(#[$a_meta])*
                $a
            ),+
        }
        impl ::std::str::FromStr for $elem {
            type Err = xso::error::Error;
            fn from_str(s: &str) -> Result<$elem, xso::error::Error> {
                Ok(match s {
                    $($b => $elem::$a),+,
                    _ => return Err(xso::error::Error::Other(concat!("Unknown value for '", $name, "' attribute.")).into()),
                })
            }
        }
        impl ::xso::FromXmlText for $elem {
            fn from_xml_text(s: String) -> Result<$elem, xso::error::Error> {
                s.parse().map_err(xso::error::Error::text_parse_error)
            }
        }
        impl ::xso::AsXmlText for $elem {
            fn as_xml_text(&self) -> Result<std::borrow::Cow<'_, str>, xso::error::Error> {
                Ok(std::borrow::Cow::Borrowed(match self {
                    $($elem::$a => $b),+
                }))
            }

            #[allow(unreachable_patterns)]
            fn as_optional_xml_text(&self) -> Result<Option<std::borrow::Cow<'_, str>>, xso::error::Error> {
                Ok(Some(std::borrow::Cow::Borrowed(match self {
                    $elem::$default => return Ok(None),
                    $($elem::$a => $b),+
                })))
            }
        }
        impl ::minidom::IntoAttributeValue for $elem {
            #[allow(unreachable_patterns)]
            fn into_attribute_value(self) -> Option<String> {
                Some(String::from(match self {
                    $elem::$default => return None,
                    $($elem::$a => $b),+
                }))
            }
        }
        impl ::std::default::Default for $elem {
            fn default() -> $elem {
                $elem::$default
            }
        }
    );
    ($(#[$meta:meta])* $elem:ident, $name:tt, ($(#[$meta_symbol:meta])* $symbol:ident => $value:tt)) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub enum $elem {
            $(#[$meta_symbol])*
            $symbol,
            /// Value when absent.
            None,
        }
        impl ::std::str::FromStr for $elem {
            type Err = xso::error::Error;
            fn from_str(s: &str) -> Result<Self, xso::error::Error> {
                Ok(match s {
                    $value => $elem::$symbol,
                    _ => return Err(xso::error::Error::Other(concat!("Unknown value for '", $name, "' attribute."))),
                })
            }
        }
        impl ::minidom::IntoAttributeValue for $elem {
            fn into_attribute_value(self) -> Option<String> {
                match self {
                    $elem::$symbol => Some(String::from($value)),
                    $elem::None => None
                }
            }
        }
        impl ::std::default::Default for $elem {
            fn default() -> $elem {
                $elem::None
            }
        }
    );
    ($(#[$meta:meta])* $elem:ident, $name:tt, bool) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub enum $elem {
            /// True value, represented by either 'true' or '1'.
            True,
            /// False value, represented by either 'false' or '0'.
            False,
        }
        impl ::std::str::FromStr for $elem {
            type Err = xso::error::Error;
            fn from_str(s: &str) -> Result<Self, xso::error::Error> {
                Ok(match s {
                    "true" | "1" => $elem::True,
                    "false" | "0" => $elem::False,
                    _ => return Err(xso::error::Error::Other(concat!("Unknown value for '", $name, "' attribute."))),
                })
            }
        }
        impl ::xso::FromXmlText for $elem {
            fn from_xml_text(s: String) -> Result<$elem, xso::error::Error> {
                match s.parse::<bool>().map_err(xso::error::Error::text_parse_error)? {
                    true => Ok(Self::True),
                    false => Ok(Self::False),
                }
            }
        }
        impl ::xso::AsXmlText for $elem {
            fn as_xml_text(&self) -> Result<::std::borrow::Cow<'_, str>, xso::error::Error> {
                match self {
                    Self::True => Ok(::std::borrow::Cow::Borrowed("true")),
                    Self::False => Ok(::std::borrow::Cow::Borrowed("false")),
                }
            }

            fn as_optional_xml_text(&self) -> Result<Option<::std::borrow::Cow<'_, str>>, xso::error::Error> {
                match self {
                    Self::True => Ok(Some(::std::borrow::Cow::Borrowed("true"))),
                    Self::False => Ok(None),
                }
            }
        }
        impl ::minidom::IntoAttributeValue for $elem {
            fn into_attribute_value(self) -> Option<String> {
                match self {
                    $elem::True => Some(String::from("true")),
                    $elem::False => None
                }
            }
        }
        impl ::std::default::Default for $elem {
            fn default() -> $elem {
                $elem::False
            }
        }
    );
    ($(#[$meta:meta])* $elem:ident, $name:tt, $type:tt, Default = $default:expr) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $elem(pub $type);
        impl ::std::str::FromStr for $elem {
            type Err = xso::error::Error;
            fn from_str(s: &str) -> Result<Self, xso::error::Error> {
                Ok($elem($type::from_str(s).map_err(xso::error::Error::text_parse_error)?))
            }
        }
        impl ::minidom::IntoAttributeValue for $elem {
            fn into_attribute_value(self) -> Option<String> {
                match self {
                    $elem($default) => None,
                    $elem(value) => Some(format!("{}", value)),
                }
            }
        }
        impl ::std::default::Default for $elem {
            fn default() -> $elem {
                $elem($default)
            }
        }
    );
}

macro_rules! generate_element_enum {
    ($(#[$meta:meta])* $elem:ident, $name:tt, $ns:ident, {$($(#[$enum_meta:meta])* $enum:ident => $enum_name:tt),+$(,)?}) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub enum $elem {
            $(
                $(#[$enum_meta])*
                $enum
            ),+
        }
        impl ::std::convert::TryFrom<minidom::Element> for $elem {
            type Error = xso::error::FromElementError;
            fn try_from(elem: minidom::Element) -> Result<$elem, xso::error::FromElementError> {
                check_ns_only!(elem, $name, $ns);
                check_no_children!(elem, $name);
                check_no_attributes!(elem, $name);
                Ok(match elem.name() {
                    $($enum_name => $elem::$enum,)+
                    _ => return Err(xso::error::Error::Other(concat!("This is not a ", $name, " element.")).into()),
                })
            }
        }
        impl From<$elem> for minidom::Element {
            fn from(elem: $elem) -> minidom::Element {
                minidom::Element::builder(
                    match elem {
                        $($elem::$enum => $enum_name,)+
                    },
                    crate::ns::$ns,
                )
                    .build()
            }
        }
    );
}

macro_rules! generate_attribute_enum {
    ($(#[$meta:meta])* $elem:ident, $name:tt, $ns:ident, $attr:tt, {$($(#[$enum_meta:meta])* $enum:ident => $enum_name:tt),+$(,)?}) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub enum $elem {
            $(
                $(#[$enum_meta])*
                $enum
            ),+
        }
        impl ::std::convert::TryFrom<minidom::Element> for $elem {
            type Error = xso::error::FromElementError;
            fn try_from(elem: minidom::Element) -> Result<$elem, xso::error::FromElementError> {
                check_ns_only!(elem, $name, $ns);
                check_no_children!(elem, $name);
                check_no_unknown_attributes!(elem, $name, [$attr]);
                Ok(match get_attr!(elem, $attr, Required) {
                    $($enum_name => $elem::$enum,)+
                    _ => return Err(xso::error::Error::Other(concat!("Invalid ", $name, " ", $attr, " value.")).into()),
                })
            }
        }
        impl From<$elem> for minidom::Element {
            fn from(elem: $elem) -> minidom::Element {
                minidom::Element::builder($name, crate::ns::$ns)
                    .attr($attr, match elem {
                         $($elem::$enum => $enum_name,)+
                     })
                     .build()
            }
        }
    );
}

macro_rules! check_self {
    ($elem:ident, $name:tt, $ns:ident) => {
        check_self!($elem, $name, $ns, $name);
    };
    ($elem:ident, $name:tt, $ns:ident, $pretty_name:tt) => {
        if !$elem.is($name, crate::ns::$ns) {
            return Err(xso::error::FromElementError::Mismatch($elem));
        }
    };
}

macro_rules! check_child {
    ($elem:ident, $name:tt, $ns:ident) => {
        check_child!($elem, $name, $ns, $name);
    };
    ($elem:ident, $name:tt, $ns:ident, $pretty_name:tt) => {
        if !$elem.is($name, crate::ns::$ns) {
            return Err(xso::error::Error::Other(
                concat!("This is not a ", $pretty_name, " element.").into(),
            )
            .into());
        }
    };
}

macro_rules! check_ns_only {
    ($elem:ident, $name:tt, $ns:ident) => {
        if !$elem.has_ns(crate::ns::$ns) {
            return Err(xso::error::Error::Other(
                concat!("This is not a ", $name, " element.").into(),
            )
            .into());
        }
    };
}

macro_rules! check_no_children {
    ($elem:ident, $name:tt) => {
        #[cfg(not(feature = "disable-validation"))]
        for _ in $elem.children() {
            return Err(xso::error::Error::Other(
                concat!("Unknown child in ", $name, " element.").into(),
            )
            .into());
        }
    };
}

macro_rules! check_no_attributes {
    ($elem:ident, $name:tt) => {
        #[cfg(not(feature = "disable-validation"))]
        for _ in $elem.attrs() {
            return Err(xso::error::Error::Other(
                concat!("Unknown attribute in ", $name, " element.").into(),
            )
            .into());
        }
    };
}

macro_rules! check_no_unknown_attributes {
    ($elem:ident, $name:tt, [$($attr:tt),*]) => (
        #[cfg(not(feature = "disable-validation"))]
        for (_attr, _) in $elem.attrs() {
            $(
                if _attr == $attr {
                    continue;
                }
            )*
            return Err(xso::error::Error::Other(concat!("Unknown attribute in ", $name, " element.")).into());
        }
    );
}

macro_rules! generate_id {
    ($(#[$meta:meta])* $elem:ident) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $elem(pub String);
        impl ::std::str::FromStr for $elem {
            type Err = xso::error::Error;
            fn from_str(s: &str) -> Result<$elem, xso::error::Error> {
                // TODO: add a way to parse that differently when needed.
                Ok($elem(String::from(s)))
            }
        }
        impl ::xso::FromXmlText for $elem {
            fn from_xml_text(s: String) -> Result<$elem, xso::error::Error> {
                Ok(Self(s))
            }
        }
        impl ::xso::AsXmlText for $elem {
            fn as_xml_text(&self) ->Result<::std::borrow::Cow<'_, str>, xso::error::Error> {
                Ok(::std::borrow::Cow::Borrowed(self.0.as_str()))
            }
        }
        impl ::minidom::IntoAttributeValue for $elem {
            fn into_attribute_value(self) -> Option<String> {
                Some(self.0)
            }
        }
    );
}

macro_rules! generate_elem_id {
    ($(#[$meta:meta])* $elem:ident, $name:tt, $ns:ident) => (
        generate_elem_id!($(#[$meta])* $elem, $name, $ns, String);
        impl ::std::str::FromStr for $elem {
            type Err = xso::error::Error;
            fn from_str(s: &str) -> Result<$elem, xso::error::Error> {
                // TODO: add a way to parse that differently when needed.
                Ok($elem(String::from(s)))
            }
        }
    );
    ($(#[$meta:meta])* $elem:ident, $name:tt, $ns:ident, $type:ty) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $elem(pub $type);
        impl ::std::convert::TryFrom<minidom::Element> for $elem {
            type Error = xso::error::FromElementError;
            fn try_from(elem: minidom::Element) -> Result<$elem, xso::error::FromElementError> {
                check_self!(elem, $name, $ns);
                check_no_children!(elem, $name);
                check_no_attributes!(elem, $name);
                // TODO: add a way to parse that differently when needed.
                Ok($elem(elem.text().parse().map_err(xso::error::Error::text_parse_error)?))
            }
        }
        impl From<$elem> for minidom::Element {
            fn from(elem: $elem) -> minidom::Element {
                minidom::Element::builder($name, crate::ns::$ns)
                    .append(elem.0.to_string())
                    .build()
            }
        }
    );
}

macro_rules! decl_attr {
    (OptionEmpty, $type:ty) => (
        Option<$type>
    );
    (Option, $type:ty) => (
        Option<$type>
    );
    (Required, $type:ty) => (
        $type
    );
    (RequiredNonEmpty, $type:ty) => (
        $type
    );
    (Default, $type:ty) => (
        $type
    );
}

macro_rules! start_decl {
    (Vec, $type:ty) => (
        Vec<$type>
    );
    (Option, $type:ty) => (
        Option<$type>
    );
    (Required, $type:ty) => (
        $type
    );
    (Present, $type:ty) => (
        bool
    );
}

macro_rules! start_parse_elem {
    ($temp:ident: Vec) => {
        let mut $temp = Vec::new();
    };
    ($temp:ident: Option) => {
        let mut $temp = None;
    };
    ($temp:ident: Required) => {
        let mut $temp = None;
    };
    ($temp:ident: Present) => {
        let mut $temp = false;
    };
}

macro_rules! do_parse {
    ($elem:ident, Element) => {
        Ok($elem)
    };
    ($elem:ident, String) => {
        Ok($elem.text())
    };
    ($elem:ident, $constructor:ident) => {
        $constructor::try_from($elem).map_err(xso::error::Error::from)
    };
}

macro_rules! do_parse_elem {
    ($temp:ident: Vec = $constructor:ident => $elem:ident, $name:tt, $parent_name:tt) => {
        match do_parse!($elem, $constructor) {
            Ok(v) => Ok($temp.push(v)),
            Err(e) => Err(e),
        }
    };
    ($temp:ident: Option = $constructor:ident => $elem:ident, $name:tt, $parent_name:tt) => {
        if $temp.is_some() {
            Err(xso::error::Error::Other(
                concat!(
                    "Element ",
                    $parent_name,
                    " must not have more than one ",
                    $name,
                    " child."
                )
                .into(),
            ))
        } else {
            match do_parse!($elem, $constructor) {
                Ok(v) => {
                    $temp = Some(v);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    };
    ($temp:ident: Required = $constructor:ident => $elem:ident, $name:tt, $parent_name:tt) => {
        if $temp.is_some() {
            Err(xso::error::Error::Other(
                concat!(
                    "Element ",
                    $parent_name,
                    " must not have more than one ",
                    $name,
                    " child."
                )
                .into(),
            ))
        } else {
            match do_parse!($elem, $constructor) {
                Ok(v) => {
                    $temp = Some(v);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    };
    ($temp:ident: Present = $constructor:ident => $elem:ident, $name:tt, $parent_name:tt) => {
        if $temp {
            Err(xso::error::Error::Other(
                concat!(
                    "Element ",
                    $parent_name,
                    " must not have more than one ",
                    $name,
                    " child."
                )
                .into(),
            ))
        } else {
            $temp = true;
            Ok(())
        }
    };
}

macro_rules! finish_parse_elem {
    ($temp:ident: Vec = $name:tt, $parent_name:tt) => {
        $temp
    };
    ($temp:ident: Option = $name:tt, $parent_name:tt) => {
        $temp
    };
    ($temp:ident: Required = $name:tt, $parent_name:tt) => {
        $temp.ok_or(xso::error::Error::Other(
            concat!("Missing child ", $name, " in ", $parent_name, " element.").into(),
        ))?
    };
    ($temp:ident: Present = $name:tt, $parent_name:tt) => {
        $temp
    };
}

macro_rules! generate_serialiser {
    ($builder:ident, $parent:ident, $elem:ident, Required, String, ($name:tt, $ns:ident)) => {
        $builder.append(
            minidom::Element::builder($name, crate::ns::$ns)
                .append(::minidom::Node::Text($parent.$elem)),
        )
    };
    ($builder:ident, $parent:ident, $elem:ident, Option, String, ($name:tt, $ns:ident)) => {
        $builder.append_all($parent.$elem.map(|elem| {
            minidom::Element::builder($name, crate::ns::$ns).append(::minidom::Node::Text(elem))
        }))
    };
    ($builder:ident, $parent:ident, $elem:ident, Option, $constructor:ident, ($name:tt, *)) => {
        $builder.append_all(
            $parent
                .$elem
                .map(|elem| ::minidom::Node::Element(minidom::Element::from(elem))),
        )
    };
    ($builder:ident, $parent:ident, $elem:ident, Option, $constructor:ident, ($name:tt, $ns:ident)) => {
        $builder.append_all(
            $parent
                .$elem
                .map(|elem| ::minidom::Node::Element(minidom::Element::from(elem))),
        )
    };
    ($builder:ident, $parent:ident, $elem:ident, Vec, $constructor:ident, ($name:tt, $ns:ident)) => {
        $builder.append_all($parent.$elem.into_iter())
    };
    ($builder:ident, $parent:ident, $elem:ident, Present, $constructor:ident, ($name:tt, $ns:ident)) => {
        $builder.append_all(
            $parent
                .$elem
                .then(|| minidom::Element::builder($name, crate::ns::$ns)),
        )
    };
    ($builder:ident, $parent:ident, $elem:ident, $_:ident, $constructor:ident, ($name:tt, $ns:ident)) => {
        $builder.append(::minidom::Node::Element(minidom::Element::from(
            $parent.$elem,
        )))
    };
}

macro_rules! generate_child_test {
    ($child:ident, $name:tt, *) => {
        $child.is($name, ::minidom::NSChoice::Any)
    };
    ($child:ident, $name:tt, $ns:tt) => {
        $child.is($name, crate::ns::$ns)
    };
}

macro_rules! generate_element {
    ($(#[$meta:meta])* $elem:ident, $name:tt, $ns:ident, attributes: [$($(#[$attr_meta:meta])* $attr:ident: $attr_action:tt<$attr_type:ty> = $attr_name:tt),+$(,)?]) => (
        generate_element!($(#[$meta])* $elem, $name, $ns, attributes: [$($(#[$attr_meta])* $attr: $attr_action<$attr_type> = $attr_name),*], children: []);
    );
    ($(#[$meta:meta])* $elem:ident, $name:tt, $ns:ident, children: [$($(#[$child_meta:meta])* $child_ident:ident: $coucou:tt<$child_type:ty> = ($child_name:tt, $child_ns:tt) => $child_constructor:ident),+$(,)?]) => (
        generate_element!($(#[$meta])* $elem, $name, $ns, attributes: [], children: [$($(#[$child_meta])* $child_ident: $coucou<$child_type> = ($child_name, $child_ns) => $child_constructor),*]);
    );
    ($(#[$meta:meta])* $elem:ident, $name:tt, $ns:ident, attributes: [$($(#[$attr_meta:meta])* $attr:ident: $attr_action:tt<$attr_type:ty> = $attr_name:tt),*$(,)?], children: [$($(#[$child_meta:meta])* $child_ident:ident: $coucou:tt<$child_type:ty> = ($child_name:tt, $child_ns:tt) => $child_constructor:ident),*$(,)?]) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        pub struct $elem {
            $(
                $(#[$attr_meta])*
                pub $attr: decl_attr!($attr_action, $attr_type),
            )*
            $(
                $(#[$child_meta])*
                pub $child_ident: start_decl!($coucou, $child_type),
            )*
        }

        impl ::xso::FromXml for $elem {
            type Builder = ::xso::minidom_compat::FromEventsViaElement<$elem>;

            fn from_events(
                qname: ::xso::exports::rxml::QName,
                attrs: ::xso::exports::rxml::AttrMap,
            ) -> Result<Self::Builder, ::xso::error::FromEventsError> {
                if qname.0 != crate::ns::$ns || qname.1 != $name {
                    return Err(::xso::error::FromEventsError::Mismatch {
                        name: qname,
                        attrs,
                    })
                }
                Self::Builder::new(qname, attrs)
            }
        }

        impl ::std::convert::TryFrom<minidom::Element> for $elem {
            type Error = xso::error::FromElementError;

            fn try_from(mut elem: minidom::Element) -> Result<$elem, xso::error::FromElementError> {
                check_self!(elem, $name, $ns);
                check_no_unknown_attributes!(elem, $name, [$($attr_name),*]);
                $(
                    start_parse_elem!($child_ident: $coucou);
                )*
                for _child in elem.take_contents_as_children() {
                    let residual = _child;
                    $(
                    // TODO: get rid of the explicit generate_child_test here
                    // once !295 has landed.
                    let residual = if generate_child_test!(residual, $child_name, $child_ns) {
                        match do_parse_elem!($child_ident: $coucou = $child_constructor => residual, $child_name, $name) {
                            Ok(()) => continue,
                            Err(other) => return Err(other.into()),
                        }
                    } else {
                        residual
                    };
                    )*
                    let _ = residual;
                    return Err(xso::error::Error::Other(concat!("Unknown child in ", $name, " element.")).into());
                }
                Ok($elem {
                    $(
                        $attr: get_attr!(elem, $attr_name, $attr_action),
                    )*
                    $(
                        $child_ident: finish_parse_elem!($child_ident: $coucou = $child_name, $name),
                    )*
                })
            }
        }

        impl From<$elem> for minidom::Element {
            fn from(elem: $elem) -> minidom::Element {
                let mut builder = minidom::Element::builder($name, crate::ns::$ns);
                $(
                    builder = builder.attr($attr_name, elem.$attr);
                )*
                $(
                    builder = generate_serialiser!(builder, elem, $child_ident, $coucou, $child_constructor, ($child_name, $child_ns));
                )*

                builder.build()
            }
        }

        impl ::xso::AsXml for $elem {
            type ItemIter<'x> = ::xso::minidom_compat::AsItemsViaElement<'x>;

            fn as_xml_iter(&self) -> Result<Self::ItemIter<'_>, ::xso::error::Error> {
                ::xso::minidom_compat::AsItemsViaElement::new(self.clone())
            }
        }
    );
}

#[cfg(test)]
macro_rules! assert_size (
    ($t:ty, $sz:expr) => (
        assert_eq!(::std::mem::size_of::<$t>(), $sz);
    );
);

// TODO: move that to src/pubsub/mod.rs, once we figure out how to use macros from there.
macro_rules! impl_pubsub_item {
    ($item:ident, $ns:ident) => {
        impl ::std::convert::TryFrom<minidom::Element> for $item {
            type Error = FromElementError;

            fn try_from(mut elem: minidom::Element) -> Result<$item, FromElementError> {
                check_self!(elem, "item", $ns);
                check_no_unknown_attributes!(elem, "item", ["id", "publisher"]);
                let mut payloads = elem.take_contents_as_children().collect::<Vec<_>>();
                let payload = payloads.pop();
                if !payloads.is_empty() {
                    return Err(Error::Other("More than a single payload in item element.").into());
                }
                Ok($item(crate::pubsub::Item {
                    id: get_attr!(elem, "id", Option),
                    publisher: get_attr!(elem, "publisher", Option),
                    payload,
                }))
            }
        }

        impl From<$item> for minidom::Element {
            fn from(item: $item) -> minidom::Element {
                minidom::Element::builder("item", ns::$ns)
                    .attr("id", item.0.id)
                    .attr("publisher", item.0.publisher)
                    .append_all(item.0.payload)
                    .build()
            }
        }

        impl ::std::ops::Deref for $item {
            type Target = crate::pubsub::Item;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::ops::DerefMut for $item {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}
