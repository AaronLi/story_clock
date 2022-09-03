use std::path::Path;
use std::{fs, thread};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use colored::Colorize;
use lazy_static::lazy_static;
use regex::{RegexBuilder, Regex};

lazy_static! {
    //static ref PARAGRAPH_PATTERN: Regex = Regex::new(r"(?P<par_text>\w[\w\W]*?)((\r?\n\r?\n)|$)").unwrap();
    static ref PARAGRAPH_PATTERN: Regex = Regex::new(r"\r?\n\r?\n").unwrap();
    static ref TIME_PATTERN: Regex = Regex::new(r"(?P<time>((?P<hour>\d[1, 2]) o'clock)|((?P<hour_g2>\d{1,2}):(?P<minute_g2>\d{2}) ?(?P<ampm_g2>[ap])(m|\.m\.)))").unwrap();
    static ref TITLE_PATTERN: Regex = Regex::new(r"Title:\s+(?P<title>(.+\n*?)+)").unwrap();
    static ref AUTHOR_PATTERN: Regex = Regex::new(r"Author:\s+(?P<author>(.+\n*?)+)").unwrap();
}

struct ParagraphInfo {
    text: String,
    book: String,
    author: String,
    section: Option<(usize, usize)>,
    time: Option<(i32, i32)>
}

impl ParagraphInfo {
    fn new(text: &str, book: &str, author: &str) -> Self {
        return ParagraphInfo{
            text: text.to_string(),
            book: book.to_string(),
            author: author.to_string(),
            section: None,
            time: None
        }
    }
}


pub async fn collect(book_path: &Path, time_path: &Path) {
    thread::scope(|s|{
        let (paragraph_tx, mut paragraph_rx) = mpsc::sync_channel(64);
        s.spawn(||{get_paragraphs(book_path, paragraph_tx)});
        let (filter_paragraph_tx, mut filter_paragraph_rx) = mpsc::sync_channel(32);
        s.spawn(move ||{paragraph_length_filter(32..512, paragraph_rx, filter_paragraph_tx)});
        let (time_paragraph_tx, mut time_paragraph_rx) = mpsc::sync_channel(16);
        s.spawn(move ||{get_time_paragraphs(filter_paragraph_rx, time_paragraph_tx)});
        s.spawn(move ||{print_time_paragraphs(time_paragraph_rx)});
    });
}

fn print_time_paragraphs(parahraphs_in: Receiver<ParagraphInfo>){
    let mut times = HashMap::new();
    while let Ok(paragraph) = parahraphs_in.recv() {
        let (text_start, text_end) = paragraph.section.unwrap();
        let (hour, minute) = paragraph.time.unwrap();
        println!("{} {}{}{}",
                 format!("{} in {} with time {}:{:02}:",
                         paragraph.author,
                         paragraph.book,
                         hour, minute)
                     .black().on_bright_white(),
                 &paragraph.text[..text_start], &paragraph.text[text_start..text_end].green(), &paragraph.text[text_end..]);
        times.insert(hour, times.get(&hour).unwrap_or(&0) + 1);
    }
    for i in 0..24 {
        println!("{}: {}", i, times.get(&i).unwrap_or(&0))
    }
}

fn paragraph_length_filter(range: Range<usize>, paragraphs_in: Receiver<ParagraphInfo>, paragraphs_out: SyncSender<ParagraphInfo>){
    while let Ok(paragraph) = paragraphs_in.recv() {
        if range.contains(&paragraph.text.len()) {
            paragraphs_out.send(paragraph);
        }
    }
}

fn get_time_paragraphs(paragraphs_in: Receiver<ParagraphInfo>, time_paragraphs_out: SyncSender<ParagraphInfo>){
    while let Ok(mut paragraph) = paragraphs_in.recv() {
        if let Some(x) = TIME_PATTERN.captures(&paragraph.text) {
            let time_text = x.name("time").unwrap();
            let mut hour = x.name("hour").unwrap_or_else(||{x.name("hour_g2").expect("Either hour or hour_g2 must be present")}).as_str().parse::<i32>().unwrap();
            let minute = x.name("minute_g2").map_or(0, |m|m.as_str().parse::<i32>().unwrap());
            if let Some(ampm) = x.name("ampm_g2") {
                if ampm.as_str() == "p" {
                    hour += 12;
                }
            }
            paragraph.section = Some((time_text.start(), time_text.end()));
            paragraph.time = Some((hour, minute));
            time_paragraphs_out.send(paragraph);
        }
    }
}

fn get_paragraphs(book_path: &Path, paragraphs_out: SyncSender<ParagraphInfo>) {
    let mut to_read = Vec::new();
    to_read.push(String::from(book_path.to_str().unwrap()));

    while let Some(path) = to_read.pop() {
        let children = fs::read_dir(&path).unwrap();
        for child in children {
            let combined_dir = child.unwrap().path();
            if combined_dir.is_file() {
                let contents = fs::read_to_string(&combined_dir).unwrap().replace('\r', "");
                let title = TITLE_PATTERN.captures(&contents).map_or(combined_dir.file_name().unwrap().to_str().unwrap(), |c|c.name("title").unwrap().as_str());
                let author = AUTHOR_PATTERN.captures(&contents).map_or("Unknown", |c|c.name("author").unwrap().as_str());
                for paragraph in PARAGRAPH_PATTERN.split(&contents) {
                    paragraphs_out.send(
                        ParagraphInfo::new(
                            paragraph,
                            title,
                                author
                        )
                    ).unwrap_or(());
                }
            } else if combined_dir.is_dir() {
                to_read.push(combined_dir.to_str().unwrap().to_string());
            }
        }
    }
}