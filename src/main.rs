extern crate core;

use std::path::Path;
use tokio;
use reqwest::{Client, Url};
use clap::{ArgGroup, Parser};

mod download_gutenberg;
mod collect_time_paragraphs;
mod pipeline;

const MAX_CHILD_TASKS: usize = 8;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("action")
    .required(true)
    .args(&["download", "collect"]),
))]
struct Cli {
    #[clap(long, short='d', action)]
    download: bool,

    #[clap(long, short='c', action)]
    collect: bool
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match (cli.download, cli.collect) {
        (true, _) => {
            download(Path::new("./books")).await;
        },
        (_, true) => {
            collect(Path::new("./books"), Path::new("./paragraphs")).await;
        },
        (_, _) => panic!("Unreachable")
    }
}

async fn collect(book_path: &Path, time_path: &Path){
    collect_time_paragraphs::collect(book_path, time_path).await;
}

async fn download(book_path: &Path){
    let base_url = "https://mirror.csclub.uwaterloo.ca/gutenberg/";
    let client = Client::new();
    download_gutenberg::download_url(base_url, book_path, client).await
}
