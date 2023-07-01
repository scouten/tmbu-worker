use std::collections::HashSet;

use chrono::{offset::FixedOffset, DateTime};
use imap::types::Fetch;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug)]
#[allow(dead_code)] // TEMPORARY while building
pub struct Message {
    date: DateTime<FixedOffset>,
    subject: String,
    link: String,
    text: String,
    tags: HashSet<String>,
}

impl Message {
    pub fn from(message: &Fetch) -> Option<Self> {
        let body = message.body().expect("Message did not have a body");
        let body = std::str::from_utf8(body)
            .expect("Message was not valid UTF-8")
            .to_string();

        let mut date: Option<DateTime<FixedOffset>> = None;
        let mut subject: Option<String> = None;
        let mut boundary: Option<String> = None;
        let mut link: Option<String> = None;
        let mut text = "".to_owned();
        let mut tags: HashSet<String> = HashSet::new();

        let mut lines = body.lines();

        loop {
            if let Some(line) = lines.next() {
                if line.is_empty() {
                    break;
                }

                if line.starts_with("Date: ") {
                    date = Some(DateTime::parse_from_rfc2822(&line[6..]).unwrap());
                }

                if line.starts_with("Subject: ") {
                    let (subj, new_tags) = extract_tags(&line[9..]);

                    subject = Some(subj.trim().to_owned());
                    tags.extend(new_tags);
                }

                if line.starts_with(" boundary=") {
                    boundary = Some(format!("--{}", (&line[10..]).to_owned()));
                }
            } else {
                break;
            }
        }

        let boundary = if let Some(boundary) = boundary {
            boundary
        } else {
            eprintln!("No boundary value found");
            return None;
        };

        loop {
            let line = if let Some(line) = lines.next() {
                line
            } else {
                break;
            };

            if line == boundary {
                if let Some(content_type) = lines.next() {
                    if content_type.to_lowercase().trim() == "content-type: text/plain" {
                        lines.next(); // ignore blank line

                        loop {
                            if let Some(line) = lines.next() {
                                if line == boundary {
                                    text = text.trim().to_owned();
                                    break;
                                } else if line.starts_with("https://") && link.is_none() {
                                    link = Some(line.to_owned());
                                } else {
                                    let (line, new_tags) = extract_tags(line);

                                    text += &line;
                                    text += "/n";

                                    tags.extend(new_tags);
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }

        if let Some(date) = date {
            if let Some(link) = link {
                return Some(Self {
                    date,
                    subject: subject.unwrap_or_default(),
                    link,
                    text: text.trim().to_owned(),
                    tags,
                });
            }
        }

        None
    }
}

fn extract_tags(s: &str) -> (String, HashSet<String>) {
    lazy_static! {
        static ref HASHTAG: Regex = Regex::new(r#"#(\w+)"#).unwrap();
    }

    let tags = HASHTAG
        .captures_iter(s)
        .map(|item| item[1].to_owned())
        .collect();
    let rem = HASHTAG.replace_all(s, "").to_string();

    (rem, tags)
}
