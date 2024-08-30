use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicI64, AtomicU64},
};

use crate::prelude::*;

pub(crate) mod other;
pub(crate) mod rolex_forums;

pub(crate) use other::*;
pub(crate) use rolex_forums::*;

#[derive(Serialize, Deserialize)]
pub struct PriceDatabase<T> {
    pub name: String,
    pub timestamp: AtomicI64,
    pub entries: Vec<T>,
}

impl<T> PriceDatabase<T>
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    pub async fn save(&self) -> Result<()> {
        use std::sync::atomic::Ordering;

        if !tokio::fs::try_exists("data/").await? {
            tokio::fs::create_dir("data/").await?;
        }

        let now_time = Utc::now().timestamp();

        self.timestamp.store(now_time, Ordering::SeqCst);

        tokio::fs::write(
            format!("data/{}.json", self.name),
            serde_json::to_string(self)?,
        )
        .await?;

        println!("Saved {} DB, {} entries", self.name, self.entries.len());

        Ok(())
    }

    pub async fn try_load(&mut self) -> Result<()> {
        let s = tokio::fs::read_to_string(format!("data/{}.json", self.name)).await?;
        *self = serde_json::from_str(&s)?;

        Ok(())
    }

    pub async fn load(&mut self) -> Result<()> {
        match self.try_load().await {
            Ok(()) => {
                println!("Loaded {} DB, {} entries", self.name, self.entries.len());

                Ok(())
            }
            Err(WatchError::IO(e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

pub trait Scraper {
    fn update(&mut self) -> AsyncResult<()>;
}
