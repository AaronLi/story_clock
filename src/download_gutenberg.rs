use regex::Regex;
use std::path::Path;
use reqwest::Client;
use tokio::task::JoinSet;
use std::fs;
use scraper::{Html, Selector};
use std::fs::File;
use std::io::Write;
use colored::Colorize;
use crate::{MAX_CHILD_TASKS, Url};
use lazy_static::lazy_static;

lazy_static! {
    static ref FOLLOW_PATTERN: Regex = Regex::new(r#"\d+/"#).unwrap();
    static ref FILE_PATTERN: Regex = Regex::new(r"^\d+.txt$").unwrap();
}

async fn make_request(url: String, client: Client) -> (Url, String) {
    let result = client.get(url).send().await.unwrap();
    return (result.url().clone(), result.text().await.unwrap());
}

pub async fn download_url(url: &str, book_directory: &Path, client: Client) {
    let mut child_tasks = JoinSet::new();
    let mut queue = Vec::new();
    child_tasks.spawn( make_request(url.to_string(), client.clone()));
    fs::create_dir_all(book_directory).unwrap_or(());
    while !child_tasks.is_empty() {
        if let Some(result) = child_tasks.join_next().await {
            let (base_url, response_body) = result.as_ref().unwrap();
            let document = Html::parse_document(response_body);
            let selector = Selector::parse("a").unwrap();
            for link in document.select(&selector) {
                let destination = link.value().attr("href");
                match destination {
                    None => {}
                    Some(dir) => {
                        let combined_path = base_url.join(dir).unwrap();
                        if dir.ends_with('/') {
                            if FOLLOW_PATTERN.is_match(dir) {
                                //println!("Following {}", combined_path);
                                if child_tasks.len() < MAX_CHILD_TASKS {
                                    child_tasks.spawn(make_request(combined_path.to_string(), client.clone()));
                                }
                                else{
                                    queue.push(combined_path.to_string());
                                }
                            }
                        } else if FILE_PATTERN.is_match(dir) {
                            println!("file {}", dir);
                            let filepath = book_directory.join(dir);
                            if !filepath.exists() {
                                let mut file = File::create(filepath);
                                match &mut file {
                                    Ok(f) => {
                                        let file_contents = client.get(combined_path).send().await.unwrap();
                                        f.write_all(file_contents.text().await.unwrap().as_bytes()).unwrap_or(());
                                        println!("{}", "Downloaded".white().on_green())
                                    }
                                    Err(e) => {
                                        println!("{}", format!("Could not write file: {}", e).black().on_red());
                                    }
                                }
                            }else{
                                println!("{}", "skipped".white().on_yellow());
                            }
                        }
                    }
                }
            }
        }
        if child_tasks.len() < MAX_CHILD_TASKS && !queue.is_empty() {
            child_tasks.spawn(make_request(queue.pop().unwrap(), client.clone()));
        }
        println!("{}", format!("Child tasks: {} Queue size: {}", child_tasks.len(), queue.len()).black().on_bright_yellow());
    }
}
