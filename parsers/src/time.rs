// Copyright (c) 2019 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use xso::{AsXml, FromXml};

use crate::date::DateTime;
use crate::iq::{IqGetPayload, IqResultPayload};
use crate::ns;
use chrono::FixedOffset;
use core::str::FromStr;
use minidom::Element;
use xso::error::{Error, FromElementError};

/// An entity time query.
#[derive(FromXml, AsXml, PartialEq, Debug, Clone)]
#[xml(namespace = ns::TIME, name = "time")]
pub struct TimeQuery;

impl IqGetPayload for TimeQuery {}

/// An entity time result, containing an unique DateTime.
#[derive(Debug, Clone)]
pub struct TimeResult(pub DateTime);

impl IqResultPayload for TimeResult {}

impl TryFrom<Element> for TimeResult {
    type Error = FromElementError;

    fn try_from(elem: Element) -> Result<TimeResult, FromElementError> {
        check_self!(elem, "time", TIME);
        check_no_attributes!(elem, "time");

        let mut tzo = None;
        let mut utc = None;

        for child in elem.children() {
            if child.is("tzo", ns::TIME) {
                if tzo.is_some() {
                    return Err(Error::Other("More than one tzo element in time.").into());
                }
                check_no_children!(child, "tzo");
                check_no_attributes!(child, "tzo");
                // TODO: Add a FromStr implementation to FixedOffset to avoid this hack.
                let fake_date = format!("{}{}", "2019-04-22T11:38:00", child.text());
                let date_time = DateTime::from_str(&fake_date).map_err(Error::text_parse_error)?;
                tzo = Some(date_time.timezone());
            } else if child.is("utc", ns::TIME) {
                if utc.is_some() {
                    return Err(Error::Other("More than one utc element in time.").into());
                }
                check_no_children!(child, "utc");
                check_no_attributes!(child, "utc");
                let date_time =
                    DateTime::from_str(&child.text()).map_err(Error::text_parse_error)?;
                match FixedOffset::east_opt(0) {
                    Some(tz) if date_time.timezone() == tz => (),
                    _ => return Err(Error::Other("Non-UTC timezone for utc element.").into()),
                }
                utc = Some(date_time);
            } else {
                return Err(Error::Other("Unknown child in time element.").into());
            }
        }

        let tzo = tzo.ok_or(Error::Other("Missing tzo child in time element."))?;
        let utc = utc.ok_or(Error::Other("Missing utc child in time element."))?;
        let date = utc.with_timezone(tzo);

        Ok(TimeResult(date))
    }
}

impl From<TimeResult> for Element {
    fn from(time: TimeResult) -> Element {
        Element::builder("time", ns::TIME)
            .append(Element::builder("tzo", ns::TIME).append(format!("{}", time.0.timezone())))
            .append(
                Element::builder("utc", ns::TIME).append(
                    time.0
                        .with_timezone(FixedOffset::east_opt(0).unwrap())
                        .format("%FT%TZ"),
                ),
            )
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // DateTime’s size doesn’t depend on the architecture.
    #[test]
    fn test_size() {
        assert_size!(TimeQuery, 0);
        assert_size!(TimeResult, 16);
    }

    #[test]
    fn parse_response() {
        let elem: Element =
            "<time xmlns='urn:xmpp:time'><tzo>-06:00</tzo><utc>2006-12-19T17:58:35Z</utc></time>"
                .parse()
                .unwrap();
        let elem1 = elem.clone();
        let time = TimeResult::try_from(elem).unwrap();
        assert_eq!(time.0.timezone(), FixedOffset::west_opt(6 * 3600).unwrap());
        assert_eq!(
            time.0,
            DateTime::from_str("2006-12-19T12:58:35-05:00").unwrap()
        );
        let elem2 = Element::from(time);
        assert_eq!(elem1, elem2);
    }
}
