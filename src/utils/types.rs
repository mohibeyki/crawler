use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct UrlData {
    pub url: Url,
    pub status: u16,
}

pub struct Database {
    pub host: String,
    pub visited: Mutex<HashSet<Url>>,
    pub urls: Mutex<Vec<UrlData>>,
    pub tx: async_channel::Sender<Url>,
    pub rx: async_channel::Receiver<Url>,
    pub worker_count: Mutex<usize>,
}
