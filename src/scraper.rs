use scc::HashSet;
use std::sync::Arc;
use url::Url;

pub async fn scrape_url(
    thread_id: usize,
    url: Url,
    tx: async_channel::Sender<Url>,
    db: Arc<HashSet<Url>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match db.insert_async(url.clone()).await {
        Ok(_) => {}
        Err(key) => {
            return Err(format!("URL [{}] is already scraped", key).into());
        }
    }

    tracing::info!("thread: {} URL: {}", thread_id, url);
    Ok(())
}
