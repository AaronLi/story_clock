extern crate core;

use std::path::Path;
use tokio;
use reqwest::{Client, Url};
use clap::{ArgGroup, Parser};
use crate::paragraph_info::ParagraphInfo;

mod download_gutenberg;
mod collect_time_paragraphs;
mod pipeline;
mod paragraph_info;
mod display_paragraph;

const MAX_CHILD_TASKS: usize = 8;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("action")
    .required(true)
    .args(&["download", "collect", "output"]),
))]
struct Cli {
    #[clap(long, short='d', action)]
    download: bool,

    #[clap(long, short='c', action)]
    collect: bool,

    #[clap(long, short='o', action)]
    output: bool
}

#[tokio::main]
async fn main() {
    let cli: Cli = Cli::parse();
    let books_path = Path::new("./books/");
    let paragraphs_path = Path::new("./paragraphs/");
    match (cli.download, cli.collect, cli.output) {
        (true, _, _) => {
            download(books_path).await;
        },
        (_, true, _) => {
            collect(books_path, paragraphs_path).await;
        },
        (_, _, true) => {
            output(paragraphs_path).await;
        }
        (_, _, _) => panic!("Unreachable")
    }
}

async fn output(time_path: &Path){
    display_paragraph::get_paragraph(time_path).await;
}

async fn collect(book_path: &Path, time_path: &Path){
    collect_time_paragraphs::collect(book_path, time_path).await;
}

async fn download(book_path: &Path){
    let base_url = "https://mirror.csclub.uwaterloo.ca/gutenberg/";
    let client = Client::new();
    download_gutenberg::download_url(base_url, book_path, client).await
}
