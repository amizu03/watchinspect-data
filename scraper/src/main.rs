#![allow(warnings)]

use futures::future::join_all;
use std::time::Duration;
use tokio::time::sleep;

mod beep;
mod brand_tokens;
mod brands;
mod currency;
mod error;
mod identify;
mod prelude;
mod scrapers;
mod tokenize;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    use prelude::*;

    // lazy_static! {
    //     static ref CONVERSION_RATES: HashMap<Box<str>, Box<[f64]>> =
    //         serde_json::from_str(&std::fs::read_to_string("./rates.json").unwrap()).unwrap();
    // }
    //
    // println!("{:?}", *CONVERSION_RATES);

    let mut scrapers: Vec<Box<dyn Scraper>> = vec![
        Box::new(OtherForum::default()),
        Box::new(RolexForums::new(ROLEX_FORUMS_ID_ROLEX_ONLY)),
        Box::new(RolexForums::new(ROLEX_FORUMS_ID_NON_ROLEX)),
    ];

    //loop {
    let results = join_all(scrapers.iter_mut().map(|s| s.update())).await;

    for r in results {
        if let Err(e) = r {
            println!("{e:?}");
        }
    }

    //sleep(Duration::from_secs(1)).await;
    //}
}
