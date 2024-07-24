// Copyright (c) 2024 xmpp-rs contributors.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use minidom::{Element, IntoAttributeValue};
use xso::{error::FromElementError, AsXml, FromXml};

use crate::ns::{self, XDATA_VALIDATE};
use crate::Error;

/// Validation Method
#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    /// … to indicate that the value(s) should simply match the field type and datatype constraints,
    /// the `<validate/>` element shall contain a `<basic/>` child element. Using `<basic/>` validation,
    /// the form interpreter MUST follow the validation rules of the datatype (if understood) and
    /// the field type.
    ///
    /// <https://xmpp.org/extensions/xep-0122.html#usercases-validation.basic>
    Basic,

    /// For "list-single" or "list-multi", to indicate that the user may enter a custom value
    /// (matching the datatype constraints) or choose from the predefined values, the `<validate/>`
    /// element shall contain an `<open/>` child element. The `<open/>` validation method applies to
    /// "text-multi" differently; it hints that each value for a "text-multi" field shall be
    /// validated separately. This effectively turns "text-multi" fields into an open-ended
    /// "list-multi", with no options and all values automatically selected.
    ///
    /// <https://xmpp.org/extensions/xep-0122.html#usercases-validation.open>
    Open,

    /// To indicate that the value should fall within a certain range, the `<validate/>` element shall
    /// contain a `<range/>` child element. The 'min' and 'max' attributes of the `<range/>` element
    /// specify the minimum and maximum values allowed, respectively.
    ///
    /// The 'max' attribute specifies the maximum allowable value. This attribute is OPTIONAL.
    /// The value depends on the datatype in use.
    ///
    /// The 'min' attribute specifies the minimum allowable value. This attribute is OPTIONAL.
    /// The value depends on the datatype in use.
    ///
    /// The `<range/>` element SHOULD possess either a 'min' or 'max' attribute, and MAY possess both.
    /// If neither attribute is included, the processor MUST assume that there are no range
    /// constraints.
    ///
    /// <https://xmpp.org/extensions/xep-0122.html#usercases-validation.range>
    Range {
        /// The 'min' attribute specifies the minimum allowable value.
        min: Option<String>,
        /// The 'max' attribute specifies the maximum allowable value.
        max: Option<String>,
    },

    /// To indicate that the value should be restricted to a regular expression, the `<validate/>`
    /// element shall contain a `<regex/>` child element. The XML character data of this element is
    /// the pattern to apply. The syntax of this content MUST be that defined for POSIX extended
    /// regular expressions, including support for Unicode. The `<regex/>` element MUST contain
    /// character data only.
    ///
    /// <https://xmpp.org/extensions/xep-0122.html#usercases-validatoin.regex>
    Regex(String),
}

/// Selection Ranges in "list-multi"
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::XDATA_VALIDATE, name = "list-range")]
pub struct ListRange {
    /// The 'min' attribute specifies the minimum allowable number of selected/entered values.
    #[xml(attribute(default))]
    pub min: Option<u32>,
    /// The 'max' attribute specifies the maximum allowable number of selected/entered values.
    #[xml(attribute(default))]
    pub max: Option<u32>,
}

/// Enum representing errors that can occur while parsing a `Datatype`.
#[derive(Debug, Clone, PartialEq)]
pub enum DatatypeError {
    /// Error indicating that a prefix is missing in the validation datatype.
    MissingPrefix {
        /// The invalid string that caused this error.
        input: String,
    },

    /// Error indicating that the validation datatype is invalid.
    InvalidType {
        /// The invalid string that caused this error.
        input: String,
    },

    /// Error indicating that the validation datatype is unknown.
    UnknownType {
        /// The invalid string that caused this error.
        input: String,
    },
}

impl std::error::Error for DatatypeError {}

impl Display for DatatypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DatatypeError::MissingPrefix { input } => {
                write!(f, "Missing prefix in validation datatype {input:?}.")
            }
            DatatypeError::InvalidType { input } => {
                write!(f, "Invalid validation datatype {input:?}.")
            }
            DatatypeError::UnknownType { input } => {
                write!(f, "Unknown validation datatype {input:?}.")
            }
        }
    }
}

/// Data Forms Validation Datatypes
///
/// <https://xmpp.org/registrar/xdv-datatypes.html>
#[derive(Debug, Clone, PartialEq)]
pub enum Datatype {
    /// A Uniform Resource Identifier Reference (URI)
    AnyUri,

    /// An integer with the specified min/max
    /// Min: -128, Max: 127
    Byte,

    /// A calendar date
    Date,

    /// A specific instant of time
    DateTime,

    /// An arbitrary-precision decimal number
    Decimal,

    /// An IEEE double-precision 64-bit floating point type
    Double,

    /// An integer with the specified min/max
    /// Min: -2147483648, Max: 2147483647
    Int,

    /// A decimal number with no fraction digits
    Integer,

    /// A language identifier as defined by RFC 1766
    Language,

    /// An integer with the specified min/max
    /// Min: -9223372036854775808, Max: 9223372036854775807
    Long,

    /// An integer with the specified min/max
    /// Min: -32768, Max: 32767
    Short,

    /// A character strings in XML
    String,

    /// An instant of time that recurs every day
    Time,

    /// A user-defined datatype
    UserDefined(String),

    /// A non-standard datatype
    Other {
        /// The prefix of the specified datatype. Should be registered with the XMPP Registrar.
        prefix: String,
        /// The actual value of the specified datatype. E.g. "lat" in the case of "geo:lat".
        value: String,
    },
}

/// Validation rules for a DataForms Field.
#[derive(Debug, Clone, PartialEq)]
pub struct Validate {
    /// The 'datatype' attribute specifies the datatype. This attribute is OPTIONAL, and defaults
    /// to "xs:string". It MUST meet one of the following conditions:
    ///
    /// - Start with "xs:", and be one of the "built-in" datatypes defined in XML Schema Part 2
    /// - Start with a prefix registered with the XMPP Registrar
    /// - Start with "x:", and specify a user-defined datatype.
    ///
    /// Note that while "x:" allows for ad-hoc definitions, its use is NOT RECOMMENDED.
    pub datatype: Option<Datatype>,

    /// The validation method. If no validation method is specified, form processors MUST
    /// assume `<basic/>` validation. The `<validate/>` element SHOULD include one of the above
    /// validation method elements, and MUST NOT include more than one.
    ///
    /// Any validation method applied to a field of type "list-multi", "list-single", or "text-multi"
    /// (other than `<basic/>`) MUST imply the same behavior as `<open/>`, with the additional constraints
    /// defined by that method.
    ///
    /// <https://xmpp.org/extensions/xep-0122.html#usecases-validation>
    pub method: Option<Method>,

    /// For "list-multi", validation can indicate (via the `<list-range/>` element) that a minimum
    /// and maximum number of options should be selected and/or entered. This selection range
    /// MAY be combined with the other methods to provide more flexibility.
    /// The `<list-range/>` element SHOULD be included only when the `<field/>` is of type "list-multi"
    /// and SHOULD be ignored otherwise.
    ///
    /// The `<list-range/>` element SHOULD possess either a 'min' or 'max' attribute, and MAY possess
    /// both. If neither attribute is included, the processor MUST assume that there are no
    /// selection constraints.
    ///
    /// <https://xmpp.org/extensions/xep-0122.html#usecases-ranges>
    pub list_range: Option<ListRange>,
}

impl TryFrom<Element> for Validate {
    type Error = FromElementError;

    fn try_from(elem: Element) -> Result<Self, Self::Error> {
        check_self!(elem, "validate", XDATA_VALIDATE);
        check_no_unknown_attributes!(elem, "item", ["datatype"]);

        let mut validate = Validate {
            datatype: elem
                .attr("datatype")
                .map(Datatype::from_str)
                .transpose()
                .map_err(|err| FromElementError::Invalid(Error::TextParseError(err.into())))?,
            method: None,
            list_range: None,
        };

        for child in elem.children() {
            match child {
                _ if child.is("list-range", XDATA_VALIDATE) => {
                    let list_range = ListRange::try_from(child.clone())?;
                    if validate.list_range.is_some() {
                        return Err(Error::Other(
                            "Encountered unsupported number (n > 1) of list-range in validate element.",
                        ).into());
                    }
                    validate.list_range = Some(list_range);
                }
                _ => {
                    let method = Method::try_from(child.clone())?;
                    if validate.method.is_some() {
                        return Err(Error::Other(
                            "Encountered unsupported number (n > 1) of validation methods in validate element.",
                        ).into());
                    }
                    validate.method = Some(method);
                }
            }
        }

        Ok(validate)
    }
}

impl From<Validate> for Element {
    fn from(value: Validate) -> Self {
        Element::builder("validate", XDATA_VALIDATE)
            .attr("datatype", value.datatype)
            .append_all(value.method)
            .append_all(value.list_range)
            .build()
    }
}

impl TryFrom<Element> for Method {
    type Error = Error;

    fn try_from(elem: Element) -> Result<Self, Self::Error> {
        let method = match elem {
            _ if elem.is("basic", XDATA_VALIDATE) => {
                check_no_attributes!(elem, "basic");
                Method::Basic
            }
            _ if elem.is("open", XDATA_VALIDATE) => {
                check_no_attributes!(elem, "open");
                Method::Open
            }
            _ if elem.is("range", XDATA_VALIDATE) => {
                check_no_unknown_attributes!(elem, "range", ["min", "max"]);
                Method::Range {
                    min: elem.attr("min").map(ToString::to_string),
                    max: elem.attr("max").map(ToString::to_string),
                }
            }
            _ if elem.is("regex", XDATA_VALIDATE) => {
                check_no_attributes!(elem, "regex");
                check_no_children!(elem, "regex");
                Method::Regex(elem.text())
            }
            _ => return Err(Error::Other("Encountered invalid validation method.")),
        };
        Ok(method)
    }
}

impl From<Method> for Element {
    fn from(value: Method) -> Self {
        match value {
            Method::Basic => Element::builder("basic", XDATA_VALIDATE),
            Method::Open => Element::builder("open", XDATA_VALIDATE),
            Method::Range { min, max } => Element::builder("range", XDATA_VALIDATE)
                .attr("min", min)
                .attr("max", max),
            Method::Regex(regex) => Element::builder("regex", XDATA_VALIDATE).append(regex),
        }
        .build()
    }
}

impl FromStr for Datatype {
    type Err = DatatypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, ":");

        let Some(prefix) = parts.next() else {
            return Err(DatatypeError::MissingPrefix {
                input: s.to_string(),
            });
        };

        match prefix {
            "xs" => (),
            "x" => {
                return Ok(Datatype::UserDefined(
                    parts.next().unwrap_or_default().to_string(),
                ))
            }
            _ => {
                return Ok(Datatype::Other {
                    prefix: prefix.to_string(),
                    value: parts.next().unwrap_or_default().to_string(),
                })
            }
        }

        let Some(datatype) = parts.next() else {
            return Err(DatatypeError::InvalidType {
                input: s.to_string(),
            });
        };

        let parsed_datatype = match datatype {
            "anyURI" => Datatype::AnyUri,
            "byte" => Datatype::Byte,
            "date" => Datatype::Date,
            "dateTime" => Datatype::DateTime,
            "decimal" => Datatype::Decimal,
            "double" => Datatype::Double,
            "int" => Datatype::Int,
            "integer" => Datatype::Integer,
            "language" => Datatype::Language,
            "long" => Datatype::Long,
            "short" => Datatype::Short,
            "string" => Datatype::String,
            "time" => Datatype::Time,
            _ => {
                return Err(DatatypeError::UnknownType {
                    input: s.to_string(),
                })
            }
        };

        Ok(parsed_datatype)
    }
}

impl Display for Datatype {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Datatype::AnyUri => "xs:anyURI",
            Datatype::Byte => "xs:byte",
            Datatype::Date => "xs:date",
            Datatype::DateTime => "xs:dateTime",
            Datatype::Decimal => "xs:decimal",
            Datatype::Double => "xs:double",
            Datatype::Int => "xs:int",
            Datatype::Integer => "xs:integer",
            Datatype::Language => "xs:language",
            Datatype::Long => "xs:long",
            Datatype::Short => "xs:short",
            Datatype::String => "xs:string",
            Datatype::Time => "xs:time",
            Datatype::UserDefined(value) => return write!(f, "x:{value}"),
            Datatype::Other { prefix, value } => return write!(f, "{prefix}:{value}"),
        };
        f.write_str(value)
    }
}

impl IntoAttributeValue for Datatype {
    fn into_attribute_value(self) -> Option<String> {
        Some(self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datatype() -> Result<(), DatatypeError> {
        assert_eq!(Datatype::AnyUri, "xs:anyURI".parse()?);
        assert_eq!(
            Err(DatatypeError::UnknownType {
                input: "xs:anyuri".to_string()
            }),
            "xs:anyuri".parse::<Datatype>(),
        );
        assert_eq!(
            "xs:".parse::<Datatype>(),
            Err(DatatypeError::UnknownType {
                input: "xs:".to_string()
            })
        );
        assert_eq!(
            Datatype::AnyUri.into_attribute_value(),
            Some("xs:anyURI".to_string())
        );

        assert_eq!(Datatype::UserDefined("id".to_string()), "x:id".parse()?);
        assert_eq!(Datatype::UserDefined("".to_string()), "x:".parse()?);
        assert_eq!(
            Datatype::UserDefined("id".to_string()).into_attribute_value(),
            Some("x:id".to_string())
        );

        assert_eq!(
            Datatype::Other {
                prefix: "geo".to_string(),
                value: "lat".to_string()
            },
            "geo:lat".parse()?
        );
        assert_eq!(
            Datatype::Other {
                prefix: "geo".to_string(),
                value: "".to_string()
            },
            "geo:".parse()?
        );
        assert_eq!(
            Datatype::Other {
                prefix: "geo".to_string(),
                value: "lat".to_string()
            }
            .into_attribute_value(),
            Some("geo:lat".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_parse_validate_element() -> Result<(), Error> {
        let cases = [
            (
                r#"<validate xmlns='http://jabber.org/protocol/xdata-validate'/>"#,
                Validate {
                    datatype: None,
                    method: None,
                    list_range: None,
                },
            ),
            (
                r#"<validate xmlns='http://jabber.org/protocol/xdata-validate' datatype="xs:string"><basic/><list-range max="3" min="1"/></validate>"#,
                Validate {
                    datatype: Some(Datatype::String),
                    method: Some(Method::Basic),
                    list_range: Some(ListRange {
                        min: Some(1),
                        max: Some(3),
                    }),
                },
            ),
            (
                r#"<validate xmlns='http://jabber.org/protocol/xdata-validate' datatype="xs:string"><regex>([0-9]{3})-([0-9]{2})-([0-9]{4})</regex></validate>"#,
                Validate {
                    datatype: Some(Datatype::String),
                    method: Some(Method::Regex(
                        "([0-9]{3})-([0-9]{2})-([0-9]{4})".to_string(),
                    )),
                    list_range: None,
                },
            ),
            (
                r#"<validate xmlns='http://jabber.org/protocol/xdata-validate' datatype="xs:dateTime"><range max="2003-10-24T23:59:59-07:00" min="2003-10-05T00:00:00-07:00"/></validate>"#,
                Validate {
                    datatype: Some(Datatype::DateTime),
                    method: Some(Method::Range {
                        min: Some("2003-10-05T00:00:00-07:00".to_string()),
                        max: Some("2003-10-24T23:59:59-07:00".to_string()),
                    }),
                    list_range: None,
                },
            ),
        ];

        for case in cases {
            let parsed_element: Validate = case
                .0
                .parse::<Element>()
                .expect(&format!("Failed to parse {}", case.0))
                .try_into()?;

            assert_eq!(parsed_element, case.1);

            let xml = String::from(&Element::from(parsed_element));
            assert_eq!(xml, case.0);
        }

        Ok(())
    }

    #[test]
    fn test_fails_with_invalid_children() -> Result<(), Error> {
        let cases = [
            r#"<validate xmlns='http://jabber.org/protocol/xdata-validate'><basic /><open /></validate>"#,
            r#"<validate xmlns='http://jabber.org/protocol/xdata-validate'><unknown /></validate>"#,
        ];

        for case in cases {
            let element = case
                .parse::<Element>()
                .expect(&format!("Failed to parse {}", case));
            assert!(Validate::try_from(element).is_err());
        }

        Ok(())
    }
}
