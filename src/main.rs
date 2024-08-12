mod utils;

use clap::Parser;
use dotenv::dotenv;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};
use url::Url;
use utils::types::{Database, UrlData};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    url: String,

    #[arg(short, long)]
    output: String,
}

#[tokio::main]
async fn main() {
    tracing::info!("starting crawler");

    dotenv().expect("failed to load dotenv");
    tracing::info!("loaded dotenv");

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

    let args = Args::parse();
    let db = Arc::new(match Url::parse(&args.url) {
        Ok(url) => {
            tracing::info!("crawler called with args url: [{url}]");

            // creating channel for communication between threads
            let (tx, rx) = async_channel::unbounded::<Url>();

            // creating a db struct to hold all the data
            let db: Database = Database {
                host: url
                    .host_str()
                    .expect("URL is valid but the host is invalid! (should not happen!)")
                    .to_string(),
                visited: Mutex::new(HashSet::<Url>::new()),
                urls: Mutex::new(Vec::<UrlData>::new()),
                tx,
                rx,
                worker_count: Mutex::new(0),
            };

            // main url has to be sent to the channel, panic if it fails
            db.tx.try_send(url).unwrap();
            db
        }
        Err(e) => {
            tracing::error!("failed to parse provided URL: {}", e);
            return;
        }
    });

    let output_file = File::create(args.output).expect("failed to create output file");

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
                utils::worker::scraper_thread(thread_id, db).await;
            })
        })
        .collect();

    for t in threads {
        t.await.unwrap();
    }

    tracing::info!(
        "all threads have exited, total URLs scraped: {}",
        db.urls.lock().unwrap().len()
    );

    let mut writer = BufWriter::new(output_file);
    let urls = db.urls.lock().unwrap();
    serde_json::to_writer_pretty(&mut writer, &*urls).expect("failed to write output file");
}
