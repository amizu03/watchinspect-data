use std::{
    fs,
    future::Future,
    io::Bytes,
    pin::Pin,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};

use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, TimeZone};
use futures::future::join_all;
use reqwest::{header::HeaderMap, Client};
use scraper::selectable::Selectable;
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
    pub fn url(&self) -> String {
        format!("https://www.rolexforums.com/showthread.php?t={}", self.id)
    }

    pub async fn update(&mut self, client: Arc<Client>) -> Result<()> {
        lazy_static! {
            static ref POST_SELECTOR: Selector =
                Selector::parse(r#"div[id^="post_message_"]"#).unwrap();
        }

        let url = self.url();

        let s = client.get(url).send().await?.text().await?;
        let doc = Html::parse_document(&s);

        for post in doc.select(&POST_SELECTOR) {
            for s in post.text() {
                let trimmed = s.trim();

                if !trimmed.is_empty() {
                    // Update is item is sold
                    let lower = trimmed.to_lowercase();

                    if lower.contains("sold") && !lower.contains("not sold") {
                        self.is_sold = true;
                    }

                    // Update price to latest price after changes
                    if let Ok(currency) = extract_currency_to_usd(self.timestamp, trimmed) {
                        self.price = Some(currency);
                    }
                }
            }
        }

        Ok(())
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

    fn fetch_page(
        client: Arc<Client>,
        forum_id: usize,
        page: usize,
    ) -> AsyncResult<(Vec<RolexForumsEntry>, usize)> {
        lazy_static! {
            static ref MOD_FORM_SELECTOR: Selector = Selector::parse("#inlinemodform").unwrap();
            static ref PAGE_NAV_SELECTOR: Selector = Selector::parse("div.pagenav").unwrap();
            static ref PAGE_NUM_SELECTOR: Selector = Selector::parse("td.vbmenu_control").unwrap();
            static ref THREAD_TABLE_SELECTOR: Selector =
                Selector::parse(r#"[id^="threadbits_forum_"]"#).unwrap();
            static ref THREAD_TABLE_ROW_SELECTOR: Selector = Selector::parse("tr").unwrap();
            static ref THREAD_TITLE_SELECTOR: Selector =
                Selector::parse(r#"[id^="thread_title_"]"#).unwrap();
            static ref ALT2_SELECTOR: Selector = Selector::parse("td.alt2").unwrap();
            static ref DATETIME_SELECTOR: Selector = Selector::parse("div.smallfont").unwrap();
            static ref TIME_SELECTOR: Selector = Selector::parse("span.time").unwrap();
        }

        Box::pin(async move {
            let url = format!(
                "https://www.rolexforums.com/forumdisplay.php?f={forum_id}&order=desc&page={page}"
            );

            let s = client.get(url).send().await?.text().await?;
            let doc = Html::parse_document(&s);
            let forum = doc.select(&MOD_FORM_SELECTOR).nth(0).unwrap();

            let page_nav = forum.select(&PAGE_NUM_SELECTOR).nth(0).unwrap();
            let page_pos_label = page_nav.text().nth(0).unwrap().split_ascii_whitespace();
            let max_page = page_pos_label.last().unwrap().parse::<usize>().unwrap();

            println!("PAGE: {page}/{max_page}");

            let thread_list_body = forum.select(&THREAD_TABLE_SELECTOR).nth(0).unwrap();
            let mut entries = Vec::new();

            for (i, tr) in thread_list_body
                .select(&THREAD_TABLE_ROW_SELECTOR)
                .enumerate()
            {
                // Skip first 3 posts on first page which are sticky threads
                if page == 1 && i < 3 {
                    continue;
                }

                let title = tr.select(&THREAD_TITLE_SELECTOR).nth(0).unwrap();
                let id = title.attr("id").unwrap()["thread_title_".len()..]
                    .parse::<u64>()
                    .unwrap();
                let title = title.text().nth(0).unwrap();
                let alt2 = tr.select(&ALT2_SELECTOR).nth(1).unwrap();
                let (date_time, time) =
                    match alt2
                        .select(&DATETIME_SELECTOR)
                        .nth(0)
                        .and_then(|date_time| {
                            date_time
                                .select(&TIME_SELECTOR)
                                .nth(0)
                                .map(|t| (date_time, t))
                        }) {
                        Some(x) => x,
                        None => continue,
                    };

                let date = date_time.text().nth(0).unwrap().trim();
                let time = time.text().nth(0).unwrap();

                let timestamp = Self::date_to_timestamp(&format!("{date} {time}"))?;
                let watch_tokens_normalized = tokenize_watch_info(title);

                match (
                    find_brand(&watch_tokens_normalized),
                    find_model_no(&watch_tokens_normalized),
                    //extract_currency_to_usd(timestamp, summary).ok(),
                ) {
                    (Ok(brand), Ok(model_no)) => {
                        if model_no.contains("$") {
                            println!("skipped bad model no");
                            continue;
                        }

                        let mut entry = RolexForumsEntry {
                            id,
                            timestamp,
                            price: None,
                            brand: brand.to_owned().into_boxed_str(),
                            is_sold: false,
                            model_no: model_no.to_owned(),
                        };

                        entries.push(entry);
                    }
                    _ => {
                        // println!("parse err");
                    }
                }

                //println!("{title}: {date} {time}");
            }

            Ok((entries, max_page))
        })
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
                position: 0,
            },
        }
    }
}

impl Scraper for RolexForums {
    fn update(&mut self) -> AsyncResult<()> {
        let forum_id = self.forum_id;
        let mut data = RolexForums::default().db;

        data.name = self.db.name.clone();
        data.timestamp = AtomicI64::new(self.db.timestamp.load(Ordering::Relaxed));
        // data.timestamp = AtomicI64::new(self.db.timestamp);
        data.position = self.db.position;

        Box::pin(async move {
            data.load().await?;

            let backup_entries = data.entries.clone();
            let mut headers = HeaderMap::new();

            headers.insert(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:127.0) Gecko/20100101 Firefox/127.0"
                    .parse()
                    .unwrap(),
            );

            // if this is the first time, start from this first page
            // or the next available
            if data.position == 0 {
                data.position = 1;
            }

            let client = Arc::new(
                ClientBuilder::new()
                    .tcp_nodelay(true)
                    //.default_headers(headers)
                    .build()?,
            );

            let mut max_page = 1000.max(data.position + 1);
            let mut unchanged_pages_sequence = 0;

            println!("Name: {}", data.name);

            while data.position <= max_page {
                // up to 3 retries
                for i in 0..3 {
                    // save progress every 10 pages in case of unexpected events
                    if data.position != 0 && data.position % 10 == 0 {
                        data.save().await?;
                    }

                    match Self::fetch_page(client.clone(), forum_id, data.position).await {
                        Ok((mut entries, new_max_page)) => {
                            let num_page_entries = entries.len();
                            let mut unchanged_entry_count = 0;

                            for mut entry in entries {
                                match data.entries.iter_mut().find(|e| e.id == entry.id) {
                                    // entry existed before, but we have new posts to parse
                                    Some(e) if e.timestamp != entry.timestamp => {
                                        *e = entry.clone();

                                        if let Err(e) = e.update(client.clone()).await {
                                            println!("{e}");
                                        } else {
                                            println!("{}", e.model_no);
                                            // println!("ok!");
                                        }
                                    }
                                    // entry exists but unchanged
                                    Some(e) => {
                                        unchanged_entry_count += 1;
                                    }
                                    // entry doesnt exist and needs to be fully parsed or newly created
                                    None => {
                                        if let Err(e) = entry.update(client.clone()).await {
                                            println!("{e}");
                                        } else {
                                            println!("{}", entry.model_no);
                                            // println!("ok!");
                                        }

                                        data.entries.push(entry);
                                    }
                                };
                            }

                            if unchanged_entry_count == num_page_entries && num_page_entries != 0 {
                                unchanged_pages_sequence += 1;
                            } else {
                                unchanged_pages_sequence = 0;
                            }

                            max_page = new_max_page;

                            break;
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            println!("RETRY {}/3", i + 1);
                            sleep(Duration::from_secs(3)).await;
                        }
                    }
                }

                data.position += 1;

                if unchanged_pages_sequence == 1 {
                    println!("Reached last changed page!");
                    break;
                }
            }

            println!("Resetting page data...");
            data.position = 0;

            println!("Removing duplicate entries...");
            data.entries.dedup_by_key(|x| x.id);

            println!("Sorting data by timestamp...");
            data.entries.sort_by_key(|x| x.timestamp);

            println!("Done!");

            // beeeeeeeep!
            beep();

            data.save().await?;

            Ok(())
        })
    }
}
