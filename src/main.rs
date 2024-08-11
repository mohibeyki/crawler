mod utils;

use clap::Parser;
use dotenv::dotenv;
use scc::HashSet;
use std::sync::Mutex;
use url::Url;
use utils::types::DB;

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
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let db: DB;

    let args = Args::parse();
    match Url::parse(&args.url) {
        Ok(url) => {
            tracing::info!("scraping URL: {}", url);

            // main url has to be sent to the channel, panic if it fails
            let (tx, rx) = async_channel::unbounded::<Url>();
            db = DB {
                host: url
                    .host_str()
                    .expect("URL is valid but the host is invalid!")
                    .to_string(),
                visited: HashSet::<Url>::new(),
                urls: Mutex::new(Vec::<Url>::new()),
                tx,
                rx,
            };
            db.tx.try_send(url).unwrap();
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

    let threads: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let db = db.clone();
            tokio::spawn(async move {
                utils::worker::scraper_thread(thread_id, &db).await;
            })
        })
        .collect();

    for t in threads {
        t.await.unwrap();
    }
    tracing::info!("all threads have exited");

    db.urls.lock().unwrap().iter().for_each(|url| {
        tracing::error!("{url}");
    });
}
