use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct UrlData {
    pub url: Url,
    pub status: u16,
}

pub struct Database {
    pub host: String,
    pub visited: Mutex<HashSet<Url>>,
    pub urls_tx: async_channel::Sender<UrlData>,
    pub tx: async_channel::Sender<Url>,
    pub rx: async_channel::Receiver<Url>,
    pub worker_count: Mutex<usize>,
}
