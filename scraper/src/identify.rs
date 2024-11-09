use crate::brand_tokens::*;
use crate::prelude::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WatchIdError {
    #[error("Failed to parse watch brand")]
    Brand,
    #[error("Failed to parse watch model number")]
    ModelNo,
    #[error("Failed to extract watch price currency")]
    Currency,
    #[error("Failed to find conversion rate")]
    ConversionRate,
}

const BAD_MODEL_NO_CHARS: [char; 2] = ['&', ')'];

pub fn find_model_no(tokens: &Box<[Box<str>]>) -> Result<&Box<str>> {
    let mut model_no = None;
    let mut best_match_count = 0;
    let mut best_char_match_count = 0;

    for t in tokens {
        if t.ends_with("mm")
            || t.ends_with("'s")
            || (t.len() == 4 && t.ends_with('s'))
            || BAD_MODEL_NO_CHARS.iter().any(|c| t.contains(*c))
        {
            continue;
        }

        let t_no_symbols = t.replace(".", "0");

        let token_len = t.len();
        let num_subsequent_chars_end = t
            .chars()
            .rev()
            .take_while(|&c| c.is_ascii_lowercase())
            .count();
        let num_subsequent_chars = t
            .chars()
            .take_while(|&c| c.is_ascii_lowercase())
            .count()
            .max(num_subsequent_chars_end);

        let num_subsequent_digits = t_no_symbols
            .chars()
            .skip(
                t_no_symbols
                    .chars()
                    .position(|x| x.is_numeric())
                    .unwrap_or(0),
            )
            .take_while(|&c| c.is_digit(10))
            .count();

        if num_subsequent_digits >= 4
            && token_len >= best_match_count
            && num_subsequent_chars >= best_char_match_count
        {
            if let Ok(num) = t_no_symbols[..num_subsequent_digits].parse::<usize>() {
                if num_subsequent_digits <= 4
                    && num >= MIN_YEAR
                    && num <= MAX_YEAR
                    && num_subsequent_chars == 0
                {
                    continue;
                }
            }

            best_char_match_count = num_subsequent_chars;
            best_match_count = token_len;

            model_no = Some(t);
        }
    }

    match model_no {
        Some(x) => Ok(x),
        _ => Err(WatchIdError::ModelNo.into()),
    }
}

pub fn find_brand(tokens: &Box<[Box<str>]>) -> Result<&'static str> {
    let mut best_brand = None;
    let mut best_match_percent = 0.0;
    let mut best_match_count = 0;
    let mut brand_match_percent = 0.0;

    for (brand, brand_tokens) in BRAND_TOKENS {
        let max_matches = brand_tokens.len();
        let matches: Vec<_> = tokens
            .iter()
            .filter(|t| brand_tokens.contains(&&***t))
            .collect();
        let num_matches = matches.len();
        let match_percent = (num_matches as f64 / max_matches as f64).min(1.0);

        if num_matches >= best_match_count && match_percent > best_match_percent {
            best_match_percent = match_percent;
            best_match_count = num_matches;
            best_brand = Some(brand);
            brand_match_percent = match_percent;
        }
    }

    match best_brand {
        Some(x) => Ok(x),
        _ => Err(WatchIdError::ModelNo.into()),
    }
}
