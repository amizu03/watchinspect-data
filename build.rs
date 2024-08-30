use brands::BRANDS;
use chrono::{Datelike, Utc};
use tokenize::tokenize_watch_info;

#[path = "src/brands.rs"]
mod brands;
#[path = "src/tokenize.rs"]
mod tokenize;

fn generate_brand_tokens() {
    let tokens: Vec<(&str, Box<[Box<str>]>)> = BRANDS
        .iter()
        .map(|s| (*s, tokenize_watch_info(s)))
        .collect();
    let tokens_len = tokens.len();

    let min_year = 1900;
    let max_year = Utc::now().year() + 1;

    let mut file = format!(
        "use crate::prelude::*;

pub const MIN_YEAR: usize = {min_year};
pub const MAX_YEAR: usize = {max_year};

pub const BRAND_TOKENS: [(&str, &[&str]); {tokens_len}] = [\n"
    );

    for (brand, t) in tokens {
        file.push_str(&format!("    ({brand:?}, &{t:?}),\n"));
    }

    file.push_str("];");

    std::fs::write("src/brand_tokens.rs", file)
        .expect("Failed to write brand tokens to src/brand_tokens.rs");
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    generate_brand_tokens();
}
