use clap::Parser;
use dotenv::dotenv;
use scc::HashSet;
use std::sync::Arc;
use url::Url;

mod scraper;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    url: String,
}

#[tokio::main]
async fn main() {
    tracing::info!("starting application");

    dotenv().ok();
    tracing::info!("loaded .env file");

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let (tx, rx) = async_channel::unbounded::<Url>();

    let db = Arc::new(HashSet::<Url>::new());

    let args = Args::parse();
    match Url::parse(&args.url) {
        Ok(url) => {
            tracing::info!("scraping URL: {}", url);

            // main url has to be sent to the channel, panic if it fails
            tx.send(url).await.unwrap();
        }
        Err(e) => {
            tracing::error!("failed to parse provided URL: {}", e);
            return;
        }
    }

    let thread_count = match std::env::var("THREAD_COUNT") {
        Ok(val) => match val.parse::<usize>() {
            Ok(val) => val,
            Err(e) => {
                tracing::error!("failed to parse THREAD_COUNT: {}", e);
                return;
            }
        },
        Err(_) => {
            tracing::warn!("THREAD_COUNT not set, defaulting to 1");
            1
        }
    };

    tracing::info!("using {} threads", thread_count);

    for id in 0..thread_count {
        let rx = rx.clone();
        let tx = tx.clone();
        let db = db.clone();

        tokio::spawn(async move {
            loop {
                let url = match rx.recv().await {
                    Ok(url) => url,
                    Err(_) => {
                        tracing::info!("channel closed, exiting");
                        return;
                    }
                };

                match scraper::scrape_url(id, url, tx.clone(), db.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("scrape failed: {}", e)
                    }
                }
            }
        });
    }

    for _ in 0..100 {
        tx.send(Url::parse("https://www.google.com").unwrap())
            .await
            .unwrap();
    }
}
