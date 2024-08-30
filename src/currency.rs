use std::str::FromStr;

use chrono::{Months, TimeZone};

use crate::prelude::*;

const CURRENCY_CODE_MAP: &[(&str, char)] = &[
    ("usd", '$'),
    ("eur", '€'),
    ("gbp", '£'),
    ("cny", '元'),
    ("jpy", '¥'),
    ("try", '₺'),
];

pub fn currency_to_symbol(s: &str) -> Option<char> {
    for &(currency, symbol) in CURRENCY_CODE_MAP {
        if s == currency {
            return Some(symbol);
        }
    }

    None
}

pub fn symbol_to_currency(s: &str) -> Option<&str> {
    for &(currency, symbol) in CURRENCY_CODE_MAP {
        if s.contains(symbol) {
            return Some(currency);
        }
    }

    None
}

pub fn extract_currency(s: &str) -> Result<(&(&str, char), String)> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"(USD|EUR|GBP|CNY|JPY|TRY|TRL|€|\$|£|元|块|¥|₺)\s?(\d{1,}(?:[.,]*\d{3})*(?:[.,]*\d*))|(\d{1,3}(?:[.,]*\d*)*(?:[.,]*\d*)?)\s?(USD|EUR|GBP|CNY|JPY|TRY|TRL|€|\$|£|元|块|¥|₺)"#).unwrap();
    }

    let s = s.to_owned().to_ascii_lowercase();

    let caps = RE.captures(&s).ok_or(WatchIdError::Currency)?;
    let caps = [
        caps.get(1).and_then(|c| Some(c.as_str())),
        caps.get(2).and_then(|c| Some(c.as_str())),
        caps.get(3).and_then(|c| Some(c.as_str())),
        caps.get(4).and_then(|c| Some(c.as_str())),
    ];

    let mut currency = caps[0].or_else(|| caps[3]).ok_or(WatchIdError::Currency)?;

    // if the currency is in symbol format, convert to 3-digit ISO-4217 format
    if let Some(c) = symbol_to_currency(currency) {
        currency = c;
    }

    let amount = caps[1].or_else(|| caps[2]).ok_or(WatchIdError::Currency)?;

    Ok((
        CURRENCY_CODE_MAP
            .iter()
            .find(|(c, _)| *c == currency)
            .ok_or(WatchIdError::Currency)?,
        amount.to_owned(),
    ))
}

pub fn extract_currency_to_usd(timestamp: i64, s: &str) -> Result<u32> {
    // load conversion rates from local store
    lazy_static! {
        static ref CONVERSION_RATES: HashMap<Box<str>, Box<[f64]>> =
            serde_json::from_str(&std::fs::read_to_string("./rates.json").unwrap()).unwrap();
    }

    // convert conversion rates store by timestamp
    lazy_static! {
        static ref CONVERSION_RATES_BY_TIMESTAMP: HashMap<Box<str>, Box<[(i64, f64)]>> =
            CONVERSION_RATES
                .iter()
                .flat_map(|(code, rates)| {
                    let rates_with_timestamps = rates
                        .iter()
                        .enumerate()
                        .flat_map(|(i_month, value)| {
                            match Utc
                                .with_ymd_and_hms(2000, 1, 1, 0, 0, 0)
                                .unwrap()
                                .checked_add_months(Months::new(i_month as u32))
                                .and_then(|d| Some(d.timestamp()))
                            {
                                Some(t) => Some((t, *value)),
                                None => None,
                            }
                        })
                        .collect::<Vec<(i64, f64)>>()
                        .into_boxed_slice();

                    Some((code.clone(), rates_with_timestamps))
                })
                .collect::<HashMap<Box<str>, Box<[(i64, f64)]>>>();
    }

    // extract currency parts (this is in any currency format. conversion to USD below.)
    let (&(code, symbol), amount) = extract_currency(s)?;

    // create currency from extracted information
    let s = format!("{symbol}{amount}");
    let mut currency = Currency::from_str(&s).or(Err(WatchIdError::Currency))?;

    // get conversion rates list for the currency
    let rates = CONVERSION_RATES_BY_TIMESTAMP
        .get(code)
        .ok_or(WatchIdError::ConversionRate)?;

    let mut best_dt = None;

    // get closest rate to the timestamp
    for &(record_timestamp, rate) in rates {
        let dt = timestamp.abs_diff(record_timestamp);

        match best_dt {
            None => best_dt = Some((dt, rate)),
            Some((current_best_dt, _)) => {
                if dt < current_best_dt {
                    best_dt = Some((dt, rate));
                }
            }
        }
    }

    // shouldn't fail. best_dt will always be Some since we fail the conversion if rates are not
    // found for the currency
    let (best_dt, best_conversion_rate) = best_dt.unwrap();

    // invert conversion rate X=USD*RATE -> USD=X/RATE -> USD=X*(1/RATE)
    let inverse_conversion_rate = 1.0 / best_conversion_rate;

    // convert to USD if not already in USD
    currency = currency.convert(inverse_conversion_rate, '$');

    // convert decimal currency value to uint whole points
    Ok(currency.value().to_u32().ok_or(WatchIdError::Currency)?)
}
