use std::{fs, future::Future, io::Bytes, pin::Pin, sync::Arc, time::Duration};

use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, TimeZone};
use futures::future::join_all;
use reqwest::{header::HeaderMap, Client};
use tokio::time::sleep;

use crate::{
    currency::extract_currency_to_usd,
    identify::{find_brand, find_model_no},
    prelude::*,
    tokenize::tokenize_watch_info,
};

pub const ROLEX_FORUMS_ID_ROLEX_ONLY: usize = 9;
pub const ROLEX_FORUMS_ID_NON_ROLEX: usize = 40;

#[derive(Serialize, Deserialize, Clone)]
pub struct RolexForumsEntry {
    pub id: u64,
    pub timestamp: i64,
    pub price: Option<u32>,
    pub is_sold: bool,
    pub brand: Box<str>,
    pub model_no: Box<str>,
}

impl RolexForumsEntry {
    pub fn link(&self) -> String {
        format!("https://www.rolexforums.com/showthread.php?t={}", self.id)
    }
}

pub struct RolexForums {
    forum_id: usize,
    db: PriceDatabase<RolexForumsEntry>,
}

impl RolexForums {
    pub fn new(forum_id: usize) -> Self {
        let mut default = Self::default();

        default.forum_id = forum_id;
        default.db.name.push_str(&format!("_{forum_id}"));

        default
    }

    fn date_to_timestamp(date_input: &str) -> Result<i64> {
        let now = Local::now();

        let datetime = if date_input.starts_with("Today") {
            // Extract time and combine with today's date
            let time_part = &date_input[6..];
            let time = NaiveTime::parse_from_str(time_part, "%I:%M %p")?;
            let date = now.date_naive();
            NaiveDateTime::new(date, time)
        } else if date_input.starts_with("Yesterday") {
            // Extract time and combine with yesterday's date
            let time_part = &date_input[10..];
            let time = NaiveTime::parse_from_str(time_part, "%I:%M %p")?;
            let date = now.date_naive().pred(); // get the date for yesterday
            NaiveDateTime::new(date, time)
        } else {
            // Handle specific date format: "21 July 2023 04:52 PM"
            NaiveDateTime::parse_from_str(date_input, "%d %B %Y %I:%M %p")?
        };

        // Convert NaiveDateTime to a DateTime<Utc>
        match Utc.from_local_datetime(&datetime) {
            chrono::offset::LocalResult::Single(t) => Ok(t.timestamp()),
            _ => Err(WatchError::ParseTime),
        }
    }

    async fn fetch_page(
        client: &Client,
        forum_id: usize,
        page: usize,
    ) -> Result<(Vec<RolexForumsEntry>, Option<usize>)> {
        let url = format!(
            "https://www.rolexforums.com/forumdisplay.php?f={forum_id}&order=desc&page={page}"
        );

        let s = client.get(url).send().await?.text().await?;

        let mut last_page_pos = s.find(r#"title="Last Page"#).unwrap_or(0);
        let max_page = if last_page_pos != 0 {
            let start = last_page_pos - 64;
            let page_needle = "page=";
            let page_pos =
                start + s[start..last_page_pos].find(page_needle).unwrap() + page_needle.len();

            s[page_pos..page_pos + s[page_pos..].find('"').unwrap()]
                .parse::<usize>()
                .ok()
        } else {
            None
        };

        if let Some(max_page) = max_page {
            println!("PAGE: {page}/{max_page}");
        }

        let mut entries = Vec::new();

        let thread_summary_needle = r#"id="td_threadtitle_"#;
        let thread_needle = r#"id="thread_title_"#;
        let title_needle = r#"title=""#;
        let time_needle = r#"<span class="time">"#;
        let mut post_i = 0;

        while let Some(offset) = s[last_page_pos..].find(thread_summary_needle) {
            last_page_pos += offset + thread_summary_needle.len();

            let end_quote_offset = s[last_page_pos..].find('"').unwrap();
            let id = s[last_page_pos..last_page_pos + end_quote_offset]
                .parse::<u64>()
                .unwrap();

            last_page_pos += end_quote_offset;

            let summary_offset = s[last_page_pos..].find(title_needle).unwrap();

            last_page_pos += summary_offset + title_needle.len();

            let title_end_offset = s[last_page_pos..].find(r#"">"#).unwrap();

            let summary = s[last_page_pos..last_page_pos + title_end_offset].trim();

            last_page_pos += title_end_offset + 2;

            let thread_title_offset = s[last_page_pos..].find(thread_needle).unwrap();
            last_page_pos += thread_title_offset + thread_needle.len();

            let thread_title_offset = s[last_page_pos..].find(r#"">"#).unwrap();
            last_page_pos += thread_title_offset + 2;
            let end_title_offset = s[last_page_pos..].find(r#"</a>"#).unwrap();

            let title = s[last_page_pos..last_page_pos + end_title_offset].trim();

            last_page_pos += end_title_offset;

            // sometimes there will be a moved thread, which don't have a time property
            // there is just a - and it fails to find the class
            // so let's just skip to the next forum post
            let time_offset = match s[last_page_pos..].find(time_needle) {
                Some(x) => x,
                None => continue,
            };

            last_page_pos += time_offset;

            let day_start_search = last_page_pos - 100;
            let day_offset = s[day_start_search..].find(r#"nowrap">"#).unwrap() + 8;
            let day = s[day_start_search + day_offset..last_page_pos].trim();

            last_page_pos += time_needle.len();

            let end_time_offset = s[last_page_pos..].find(r#"</span>"#).unwrap();
            let time = s[last_page_pos..last_page_pos + end_time_offset].trim();

            last_page_pos += end_time_offset;

            // skip first 3 posts (they are sticky)
            if page != 1 || post_i >= 3 {
                let timestamp = Self::date_to_timestamp(&format!("{day} {time}"))?;

                let watch_tokens_normalized = tokenize_watch_info(title);

                match (
                    find_brand(&watch_tokens_normalized),
                    find_model_no(&watch_tokens_normalized),
                    extract_currency_to_usd(timestamp, summary).ok(),
                ) {
                    (Ok(brand), Ok(model_no), price) => {
                        entries.push(RolexForumsEntry {
                            id,
                            timestamp,
                            price,
                            brand: brand.to_owned().into_boxed_str(),
                            is_sold: false,
                            model_no: model_no.to_owned(),
                        });

                        match price {
                            Some(price) => {
                                println!("{id}: ${:.2} {brand}, {model_no}", price as f64 / 100.0)
                            }
                            None => println!("{id}: None {brand}, {model_no}"),
                        }
                    }
                    _ => {}
                }
            }

            post_i += 1;
        }

        Ok((entries, max_page))
    }
}

impl Default for RolexForums {
    fn default() -> Self {
        Self {
            forum_id: 0,
            db: PriceDatabase {
                name: "RolexForums".to_owned(),
                timestamp: 0.into(),
                entries: Vec::new(),
            },
        }
    }
}

impl Scraper for RolexForums {
    fn update(&mut self) -> AsyncResult<()> {
        let forum_id = self.forum_id;

        Box::pin(async move {
            let mut data = RolexForums::default().db;

            data.load().await?;

            let mut headers = HeaderMap::new();

            headers.insert(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:127.0) Gecko/20100101 Firefox/127.0"
                    .parse()
                    .unwrap(),
            );

            let mut page = 1;

            let client = ClientBuilder::new()
                .tcp_nodelay(true)
                //.default_headers(headers)
                .build()?;

            let mut max_page = 1000;

            while page <= max_page {
                // up to 3 retries
                for i in 0..3 {
                    match Self::fetch_page(&client, forum_id, page).await {
                        Ok((mut entries, new_max_page)) => {
                            data.entries.append(&mut entries);

                            if let Some(new_max_page) = new_max_page {
                                max_page = new_max_page;
                            }

                            break;
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            println!("RETRY {}/3", i + 1);
                            sleep(Duration::from_secs(3)).await;
                        }
                    }
                }

                page += 1;
            }

            println!("Removing duplicate entries...");
            data.entries.dedup_by_key(|x| x.id);

            println!("Sorting data by timestamp...");
            data.entries.sort_by_key(|x| x.timestamp);

            println!("Done!");

            data.save().await?;

            Ok(())
        })
    }
}
