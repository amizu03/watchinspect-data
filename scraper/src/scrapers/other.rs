use std::sync::Arc;
use std::{future::Future, pin::Pin};

use crate::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct OtherEntry {
    pub post_id: u64,
    pub timestamp: i64,
    pub is_sold: bool,
}

#[repr(transparent)]
pub struct OtherForum(PriceDatabase<OtherEntry>);

impl Default for OtherForum {
    fn default() -> Self {
        Self(PriceDatabase {
            name: "OtherForum".to_owned(),
            timestamp: 0.into(),
            entries: Vec::new(),
            position: 0,
        })
    }
}

impl Scraper for OtherForum {
    fn update(&mut self) -> AsyncResult<()> {
        Box::pin(async { Ok(()) })
    }
}
