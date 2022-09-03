use std::path::Path;
use std::fs;
use std::fs::DirEntry;
use rand::{Rng, thread_rng};
use chrono::Timelike;
use colored::Colorize;
use crate::ParagraphInfo;

pub async fn get_paragraph(time_path: &Path){
    let current_time = chrono::offset::Local::now();
    display_time(time_path, current_time.hour(), current_time.minute());
}

fn display_time(time_path: &Path, hour: u32, minute: u32) {
    let paragraph_path = time_path.join(format!("{}/", hour));
    let minute_s = format!("{}/", minute);
    let mut minute_folder = Path::new(&minute_s);
    if !paragraph_path.join(minute_folder).exists() {
        minute_folder = Path::new("0/");
    }
    let paragraph_path = paragraph_path.join(minute_folder);
    let paragraphs = fs::read_dir(paragraph_path).unwrap().filter(|v|{v.is_ok()}).map(|v|{v.unwrap()}).collect::<Vec<DirEntry>>();
    let paragraph_file = &paragraphs[thread_rng().gen::<usize>() % paragraphs.len()];
    let file_contents = fs::read_to_string(&paragraph_file.path()).unwrap();
    let paragraph = serde_yaml::from_str::<ParagraphInfo>(&file_contents).unwrap();
    let (start, end) = paragraph.section.unwrap();
    println!("{}{}{}", &paragraph.text[..start], &paragraph.text[start..end].green(), &paragraph.text[end..]);
    println!("\t\t\t\t{}", format!("{} in {}", paragraph.author, paragraph.book).black().on_bright_white());
}
