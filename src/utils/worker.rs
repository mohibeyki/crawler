use scraper::{Html, Selector};
use url::Url;

use super::types::DB;

async fn extract_urls(
    base_url: Url,
    body: String,
    tx: async_channel::Sender<Url>,
) -> Result<(), Box<dyn std::error::Error>> {
    let doc = Html::parse_document(&body);
    let selector = Selector::parse("a[href]").unwrap();

    doc.select(&selector).for_each(|element| {
        if let Some(href) = element.value().attr("href") {
            match base_url.join(href) {
                Ok(url) => match tx.try_send(url.clone()) {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::warn!("failed to send URL to channel: [{url}] error: [{err}]");
                    }
                },
                Err(e) => {
                    tracing::warn!("found invalid URL: {e}");
                }
            }
        }
    });

    Ok(())
}

async fn scrape_url(
    thread_id: usize,
    url: Url,
    client: &reqwest::Client,
    db: &DB,
) -> Result<(), Box<dyn std::error::Error>> {
    match db.visited.insert_async(url.clone()).await {
        Ok(_) => {}
        Err(key) => {
            return Err(format!("URL [{key}] is already scraped").into());
        }
    }

    tracing::info!("thread: {} URL: {}", thread_id, url);

    match client.get(url.clone()).send().await {
        Ok(res) => {
            if res.status().is_client_error() || res.status().is_server_error() {
                return Err(
                    format!("failed to fetch URL [{}] status: [{}]", url, res.status()).into(),
                );
            }

            // add to list of urls
            db.urls.lock().unwrap().push(url.clone());

            // if the host is not the same as the main host, don't scrape
            if url.host_str().unwrap() != db.host {
                return Ok(());
            }

            // extract URLs from the response body
            extract_urls(url, res.text().await?, db.tx.clone()).await?;
        }
        Err(e) => {
            return Err(e.into());
        }
    }

    Ok(())
}

pub async fn scraper_thread(thread_id: usize, db: &DB) {
    let client = reqwest::Client::new();

    loop {
        let url = match db.rx.recv().await {
            Ok(url) => url,
            Err(_) => {
                tracing::info!("channel closed, exiting");
                return;
            }
        };

        match scrape_url(thread_id, url, &client, db).await {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("scrape failed: {e:?}")
            }
        }
    }
}
