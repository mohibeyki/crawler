use scc::HashSet;
use std::sync::Mutex;
use url::Url;

pub struct DB {
    pub host: String,
    pub visited: HashSet<Url>,
    pub urls: Mutex<Vec<Url>>,
    pub tx: async_channel::Sender<Url>,
    pub rx: async_channel::Receiver<Url>,
}

impl Clone for DB {
    fn clone(&self) -> Self {
        DB {
            host: self.host.clone(),
            visited: self.visited.clone(),
            urls: Mutex::new(self.urls.lock().unwrap().clone()),
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}

