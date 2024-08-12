use scraper::{Html, Selector};
use std::sync::Arc;
use url::Url;

use super::types::{Database, UrlData};

async fn extract_urls(
    base_url: Url,
    body: String,
    tx: async_channel::Sender<Url>,
) -> Result<(), Box<dyn std::error::Error>> {
    let doc = Html::parse_document(&body);
    let selector = Selector::parse("a[href]").unwrap();
    let mut result: Result<(), Box<dyn std::error::Error>> = Ok(());

    doc.select(&selector).for_each(|element| {
        if let Some(href) = element.value().attr("href") {
            match base_url.join(href) {
                Ok(url) => {
                    if let Err(err) = tx.try_send(url.clone()) {
                        result = Err(err.clone().into());
                        tracing::error!(
                            "failed to send URL to channel: [{}] error: [{}]",
                            url,
                            err
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!("found invalid URL: {}", err);
                }
            }
        }
    });

    result
}

async fn scrape_url(
    thread_id: usize,
    url: Url,
    client: &reqwest::Client,
    db: Arc<Database>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::trace!("thread: {} URL: {}", thread_id, url);

    match client.get(url.clone()).send().await {
        Ok(res) => {
            // add to list of urls
            {
                let mut urls = db.urls.lock().unwrap();
                urls.push(UrlData {
                    url: url.clone(),
                    status: res.status().as_u16(),
                });
            }

            // if the request fails, don't scrape
            if res.status().is_client_error() || res.status().is_server_error() {
                return Ok(());
            }

            // if the host is not the same as the main host, don't scrape
            if url.host_str().unwrap_or_default() != db.host {
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

pub async fn scraper_thread(thread_id: usize, db: Arc<Database>) {
    let client = reqwest::Client::new();

    loop {
        let url = match db.rx.recv().await {
            Ok(url) => {
                // check if the URL has already been visited
                {
                    let mut visited = db.visited.lock().unwrap();
                    if visited.contains(&url) {
                        continue;
                    }
                    visited.insert(url.clone());
                }

                // increment the worker count
                {
                    let mut worker_count = db.worker_count.lock().unwrap();
                    *worker_count += 1;
                }

                url
            }
            Err(_) => {
                tracing::trace!("channel closed, exiting");
                return;
            }
        };

        match scrape_url(thread_id, url, &client, db.clone()).await {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("scrape failed: {e:?}")
            }
        }

        // decrement the worker count
        let mut worker_count = db.worker_count.lock().unwrap();
        *worker_count -= 1;

        // if the channel is empty and no workers are active, close the channel
        if db.rx.is_empty() && !db.tx.is_closed() && *worker_count == 0 {
            tracing::info!("input channel is empty and no workers are active, closing the channel");
            db.tx.close();
        }
    }
}
