use serde::{Serialize, Deserialize};

#[derive(Clone,Serialize, Deserialize)]
pub struct ParagraphInfo {
    pub text: String,
    pub book: String,
    pub author: String,
    pub section: Option<(usize, usize)>,
    pub time: Option<(i32, i32)>
}

impl ParagraphInfo {
    pub fn new(text: &str, book: &str, author: &str) -> Self {
        return ParagraphInfo{
            text: text.to_string(),
            book: book.to_string(),
            author: author.to_string(),
            section: None,
            time: None
        }
    }
}
