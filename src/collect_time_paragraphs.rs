use std::path::{Path, PathBuf};
use std::fs;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::ops::Range;
use std::sync::mpsc::{Receiver, SyncSender};
use colored::Colorize;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use crate::paragraph_info::ParagraphInfo;
use crate::pipeline::{Pipeline, PipelineConsumer, PipelineIntermediateStage, PipelineProducer, PipelineStageConsumer, PipelineStageProducer, PipelineStep};

lazy_static! {
    //static ref PARAGRAPH_PATTERN: Regex = Regex::new(r"(?P<par_text>\w[\w\W]*?)((\r?\n\r?\n)|$)").unwrap();
    static ref PARAGRAPH_PATTERN: Regex = Regex::new(r"\r?\n\r?\n").unwrap();
    static ref TIME_PATTERN: Regex = RegexBuilder::new(r"(?P<time>((?P<hour_oclock>\d{1, 2}) o'clock)|((?P<hour_ampm>\d{1,2}):(?P<minute_ampm>\d{2}) ?(?P<ampm>[ap]) ?(m|\.m\.)))").case_insensitive(true).build().unwrap();
    static ref TITLE_PATTERN: Regex = Regex::new(r"Title:\s+(?P<title>(.+\n*?)+)").unwrap();
    static ref AUTHOR_PATTERN: Regex = Regex::new(r"Author:\s+(?P<author>(.+\n*?)+)").unwrap();
}


pub async fn collect(book_path: &Path, time_path: &Path) {
    let mut pipeline: Pipeline<ParagraphInfo> = Pipeline::new();
    pipeline.set_source(ParagraphProducer::new(book_path));
    pipeline.push_stage(ParagraphLengthFilter::new(32..400));
    pipeline.push_stage(ParagraphTimeFilter::new());
    pipeline.push_stage(ParagraphPrinter::new());
    pipeline.set_sink(ParagraphSaver::new(time_path));
    pipeline.execute();
}
struct ParagraphSaver {
    time_path: PathBuf
}

impl PipelineStageConsumer<ParagraphInfo> for ParagraphSaver {
    fn execute(&self, input_channel: Receiver<ParagraphInfo>) {
        fs::create_dir_all(&self.time_path).unwrap();
        while let Ok(paragraph) = input_channel.recv() {
            let (hour, minute) = paragraph.time.unwrap();
            let time_path = self.time_path.join(format!("{}/{}/", hour, minute));
            fs::create_dir_all(&time_path).unwrap();
            let yaml = serde_yaml::to_string(&paragraph).unwrap();
            let mut filename_hasher = DefaultHasher::new();
            let (start, end) = paragraph.section.unwrap();
            filename_hasher.write(paragraph.author.as_bytes());
            filename_hasher.write_usize(start);
            filename_hasher.write_usize(end);
            filename_hasher.write(paragraph.text.as_bytes());
            let file_name = format!("{}.yaml", filename_hasher.finish());
            let file_path = time_path.join(file_name);
            fs::write(file_path, &yaml).unwrap_or(());
        }
    }

    fn get_input_buffer_size(&self) -> usize {
        8
    }
}

impl PipelineConsumer<ParagraphInfo> for ParagraphSaver{}

impl ParagraphSaver {
    fn new(data_path: &Path) -> Self{
        ParagraphSaver{
            time_path: data_path.to_path_buf()
        }
    }
}

struct ParagraphPrinter {}

impl PipelineIntermediateStage<ParagraphInfo> for ParagraphPrinter {
    fn execute(&self, input_channel: Receiver<ParagraphInfo>, output_channel: SyncSender<ParagraphInfo>) {
        let mut times = HashMap::new();
        while let Ok(paragraph) = input_channel.recv() {
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
            output_channel.send(paragraph).unwrap_or(());
        }
        for i in 0..=24 {
            println!("{}: {}", i, times.get(&i).unwrap_or(&0))
        }
    }

    fn get_input_buffer_size(&self) -> usize {
        8
    }
}

impl ParagraphPrinter {
    fn new() -> Self {
        ParagraphPrinter{}
    }
}
impl PipelineStep<ParagraphInfo> for ParagraphPrinter {}

impl PipelineStageConsumer<ParagraphInfo> for ParagraphPrinter {
    fn execute(&self, input_channel: Receiver<ParagraphInfo>) {
        let mut times = HashMap::new();
        while let Ok(paragraph) = input_channel.recv() {
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
        for i in 0..=24 {
            println!("{}: {}", i, times.get(&i).unwrap_or(&0))
        }
    }

    fn get_input_buffer_size(&self) -> usize {
        8
    }
}

impl PipelineConsumer<ParagraphInfo> for ParagraphPrinter{}
struct ParagraphTimeFilter {}

impl PipelineIntermediateStage<ParagraphInfo> for ParagraphTimeFilter {
    fn execute(&self, input_channel: Receiver<ParagraphInfo>, output_channel: SyncSender<ParagraphInfo>) {
        while let Ok(mut paragraph) = input_channel.recv() {
            for x in TIME_PATTERN.captures_iter(&paragraph.text) {
                let time_text = x.name("time").unwrap();
                let hour_str = x.name("hour_oclock").unwrap_or_else(||{x.name("hour_ampm").expect("Either hour or hour_g2 must be present")}).as_str();
                let mut hour = hour_str.parse::<i32>().expect(&format!("failed to parse \"{}\"", hour_str));
                let minute = x.name("minute_ampm").map_or(0, |m|m.as_str().parse::<i32>().unwrap());
                if !(0..=24).contains(&hour) || !(0..60).contains(&minute) {
                    continue
                }
                if let Some(ampm) = x.name("ampm") {
                    if ampm.as_str() == "p" {
                        hour += 12;
                    }
                }
                paragraph.time = Some((hour, minute));
                paragraph.section = Some((time_text.start(), time_text.end()));
                output_channel.send(paragraph.clone()).unwrap_or(());
                if x.name("hour_oclock").is_some() {
                    // oclock time can be am or pm
                    paragraph.time = Some(((hour + 12) % 24, minute));
                    paragraph.section = Some((time_text.start(), time_text.end()));
                    output_channel.send(paragraph.clone()).unwrap_or(());
                }
            }
        }
    }

    fn get_input_buffer_size(&self) -> usize {
        32
    }
}

impl ParagraphTimeFilter{
    fn new() -> Self {
        ParagraphTimeFilter{}
    }
}

impl PipelineStep<ParagraphInfo> for ParagraphTimeFilter{}

struct ParagraphLengthFilter {
    length_range: Range<usize>
}

impl PipelineIntermediateStage<ParagraphInfo> for ParagraphLengthFilter {
    fn execute(&self, input_channel: Receiver<ParagraphInfo>, output_channel: SyncSender<ParagraphInfo>) {
        while let Ok(paragraph) = input_channel.recv() {
            if self.length_range.contains(&paragraph.text.len()) {
                output_channel.send(paragraph).unwrap_or(());
            }
        }
    }

    fn get_input_buffer_size(&self) -> usize {
        64
    }
}

impl ParagraphLengthFilter {
    fn new(range: Range<usize>) -> Self {
        ParagraphLengthFilter{
            length_range: range
        }
    }
}

impl PipelineStep<ParagraphInfo> for ParagraphLengthFilter {}

struct ParagraphProducer {
    // reads paragraphs from a file
    book_path: PathBuf
}

impl PipelineStageProducer<ParagraphInfo> for ParagraphProducer {
    fn execute(&self, output_channel: SyncSender<ParagraphInfo>) {
        let mut to_read = Vec::new();
        to_read.push(String::from(self.book_path.to_str().unwrap()));

        while let Some(path) = to_read.pop() {
            let children = fs::read_dir(&path).unwrap();
            for child in children {
                let combined_dir = child.unwrap().path();
                if combined_dir.is_file() {
                    let contents = fs::read_to_string(&combined_dir).unwrap().replace('\r', "");
                    let title = TITLE_PATTERN.captures(&contents).map_or(combined_dir.file_name().unwrap().to_str().unwrap(), |c|c.name("title").unwrap().as_str());
                    let author = AUTHOR_PATTERN.captures(&contents).map_or("Unknown", |c|c.name("author").unwrap().as_str());
                    for paragraph in PARAGRAPH_PATTERN.split(&contents) {
                        output_channel.send(
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
}

impl PipelineProducer<ParagraphInfo> for ParagraphProducer {

}
impl ParagraphProducer {
    fn new(book_path: &Path) -> Self{
        ParagraphProducer{
            book_path: book_path.to_path_buf()
        }
    }
}