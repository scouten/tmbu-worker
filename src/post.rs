use std::{
    collections::HashSet, env, fs, fs::File, io, io::Write, path::PathBuf, process::Command,
};

use chrono::{offset::FixedOffset, DateTime, Datelike};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use titlecase::titlecase;

use crate::{message::Message, read_line::ReadLine};

#[derive(Debug)]
#[allow(dead_code)] // TEMPORARY while building
pub struct Post {
    date: DateTime<FixedOffset>,
    subject: String,
    link: Option<String>,
    text: String,
    tags: HashSet<String>,
    via: Option<String>,
}

impl Post {
    pub fn from(message: Message) -> Self {
        Self {
            date: message.date,
            subject: message.subject,
            link: message.link,
            text: message.text,
            tags: message.tags,
            via: None,
        }
    }

    pub fn update_if_mastodon_link(&mut self) {
        // If the link is a Mastodon post, read it and
        // update the link and text accordingly.

        // If no link, nothing to do here.
        let link = match self.link {
            Some(ref link) => link.to_owned(),
            None => {
                return;
            }
        };

        let client = reqwest::blocking::Client::builder().build().unwrap();

        let note: MastodonNote = match client
            .get(&link)
            .header("Accept", "application/activity+json")
            .send()
            .and_then(|r| r.json())
        {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Unable to follow post link {link}\n\n{e}\n");
                return;
            }
        };

        let user: MastodonUser = match note.attributed_to {
            Some(ref att) => {
                match client
                    .get(att)
                    .header("Accept", "application/activity+json")
                    .send()
                    .and_then(|r| r.json())
                {
                    Ok(resp) => resp,
                    Err(e) => {
                        panic!("Unable to follow user link {link}\n\n{e}\n", link = att);
                    }
                }
            }
            None => {
                // Argh. Some Mastodon instances don't provide user info.

                if link.starts_with("https://botsin.space/@RustTrending/") {
                    MastodonUser {
                        name: "Rust Trending".to_owned(),
                    }
                } else {
                    panic!("No known user for link {link}");
                }
            }
        };

        // OK, this is definitely a Mastodon post link.
        // See if it has a link to another page.
        let mut user_comment = note.content.clone();
        lazy_static! {
            static ref A_HREF: Regex = Regex::new(r#"<a href="([^"]*)".*>.*</a>"#).unwrap();
        }

        if let Some(link_capture) = A_HREF.captures(&user_comment) {
            let href = link_capture[1].to_owned();
            self.link = Some(href.to_string());

            user_comment = A_HREF.replace(&user_comment, "").to_string();
        }

        lazy_static! {
            static ref P: Regex = Regex::new(r#"</?p>"#).unwrap();
        }

        user_comment = P.replace_all(&user_comment, "\n").trim().to_string();

        // Update pending Zola post.
        self.via = Some("Mastodon".to_owned());

        let mut text = self.text.clone();
        text = text.replace(&link, "");
        text = text.trim().to_owned();

        self.text = format!(
            "via [{user_name}]({link}): {user_comment}\n\n{text}",
            user_name = user.name,
        )
        .trim()
        .to_owned();
    }

    pub fn update_if_unsigned_mastodon_link(&mut self) {
        // If the link is a Mastodon post from a server that
        // requires signed requests, read it and update the link
        // and text accordingly.

        // If no link, nothing to do here.
        let link = match self.link {
            Some(ref link) => link.to_owned(),
            None => {
                return;
            }
        };

        let client = reqwest::blocking::Client::builder().build().unwrap();

        let post = match client
            .get(&link)
            .header("Accept", "text/html")
            .send()
            .and_then(|r| r.text())
        {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Unable to follow post link as HTML {link}\n\n{e}\n");
                return;
            }
        };

        dbg!(&post);

        let user = if link.starts_with("https://botsin.space/@RustTrending/") {
            MastodonUser {
                name: "Rust Trending".to_owned(),
            }
        } else {
            eprintln!("Need user for link {link}");
            return;
        };

        // Attempt to pull post content out of meta content header.

        lazy_static! {
            static ref META_CONTENT: Regex =
                Regex::new(r#"<meta content='([^']*)' name='description'>"#).unwrap();
        }

        let mut user_comment = if let Some(content_capture) = META_CONTENT.captures(&post) {
            content_capture[1].to_owned()
        } else {
            return;
        };

        // OK, this is likely a Mastodon post link.
        // See if it has a link to another page.
        lazy_static! {
            static ref A_HREF: Regex = Regex::new(r#"(https://[^ ]*)"#).unwrap();
        }

        if let Some(link_capture) = A_HREF.captures(&user_comment) {
            let href = link_capture[1].to_owned();
            self.link = Some(href.to_string());

            user_comment = A_HREF.replace(&user_comment, "").to_string();
        }

        lazy_static! {
            static ref P: Regex = Regex::new(r#"</?p>"#).unwrap();
        }

        user_comment = P.replace_all(&user_comment, "\n").trim().to_string();

        // Update pending Zola post.
        self.via = Some("Mastodon".to_owned());

        let mut text = self.text.clone();
        text = text.replace(&link, "");
        text = text.trim().to_owned();

        self.text = format!(
            "via [{user_name}]({link}): {user_comment}\n\n{text}",
            user_name = user.name,
        )
        .trim()
        .to_owned();
    }

    pub fn add_link_text(&mut self) {
        // If post contains a link, grab its title and
        // add that link to the end of the post message.

        // If no link, nothing to do here.
        let link = match self.link {
            Some(ref link) => link.to_owned(),
            None => {
                return;
            }
        };

        let body = match reqwest::blocking::get(&link).and_then(|r| r.text()) {
            Ok(body) => body,
            Err(e) => {
                eprintln!("Unable to follow post link {link}\n\n{e}\n");
                return;
            }
        };

        lazy_static! {
            static ref TITLE: Regex = Regex::new(r#"<title>(.*)</title>"#).unwrap();
        }

        let mut title = if let Some(link_capture) = TITLE.captures(&body) {
            link_capture[1].to_owned()
        } else {
            link.to_owned()
        };

        let site_name: Option<&str> = if title.starts_with("GitHub - ") {
            title = title.replace("GitHub - ", "");
            Some("GitHub")
        } else {
            None
        };

        let site_name = site_name.map_or("".to_owned(), |s| format!("{s}: "));

        self.text = format!("{text}\n\n{site_name}[{title}]({link})", text = self.text);
    }

    pub fn capitalize_tags(&mut self) {
        self.tags = self
            .tags
            .iter()
            .map(|tag| {
                let mut tag = titlecase(tag);
                match tag.as_str() {
                    "1password" => {
                        tag = "1Password".to_owned();
                    }
                    "Activitypub" => {
                        tag = "ActivityPub".to_owned();
                    }
                    "Aqi" => {
                        tag = "AQI".to_owned();
                    }
                    "Aws" => {
                        tag = "AWS".to_owned();
                    }
                    "Cicd" => {
                        tag = "CICD".to_owned();
                    }
                    "Cli" => {
                        tag = "CLI".to_owned();
                    }
                    "Crdt" => {
                        tag = "CRDT".to_owned();
                    }
                    "Css" => {
                        tag = "CSS".to_owned();
                    }
                    "Cta" => {
                        tag = "CTA".to_owned();
                    }
                    "Git" => {
                        tag = "git".to_owned();
                    }
                    "Github" => {
                        tag = "GitHub".to_owned();
                    }
                    "Githubactions" => {
                        tag = "GitHubActions".to_owned();
                    }
                    "Html" => {
                        tag = "HTML".to_owned();
                    }
                    "Ios" => {
                        tag = "iOS".to_owned();
                    }
                    "Iphone" => {
                        tag = "iPhone".to_owned();
                    }
                    "Javascript" => {
                        tag = "JavaScript".to_owned();
                    }
                    "Oss" => {
                        tag = "OSS".to_owned();
                    }
                    "Pdf" => {
                        tag = "PDF".to_owned();
                    }
                    "Sast" => {
                        tag = "SAST".to_owned();
                    }
                    "Sbom" => {
                        tag = "SBOM".to_owned();
                    }
                    "Sql" => {
                        tag = "SQL".to_owned();
                    }
                    "Sqlite" => {
                        tag = "SQLite".to_owned();
                    }
                    "Usb" => {
                        tag = "USB".to_owned();
                    }
                    "Usb-C" => {
                        tag = "USB-C".to_owned();
                    }
                    "Vscode" => {
                        tag = "VSCode".to_owned();
                    }
                    "Wasm" => {
                        tag = "WASM".to_owned();
                    }
                    _ => (),
                };
                tag
            })
            .collect();
    }

    pub fn render(&self) {
        let zola_path = env::var("TMBU_ZOLA_ROOT").unwrap();

        let date = self.date.date_naive();

        let mut page_path = PathBuf::from(&zola_path);
        page_path = page_path.join("content");
        page_path = page_path.join(date.year().to_string());
        page_path = page_path.join(format!("{month:02}", month = date.month()));
        fs::create_dir_all(&page_path).unwrap();

        page_path = page_path.join(format!(
            "{day:02}-{slug}.md",
            day = date.day(),
            slug = slug_from_title(&self.subject)
        ));

        println!("\nCreating blog post at {page_path:#?}");

        let mut md = File::create(&page_path).unwrap();
        writeln!(md, "+++").unwrap();
        writeln!(md, "title = {title:#?}", title = self.subject).unwrap();
        writeln!(md, "date = {date:#?}", date = self.date).unwrap();
        writeln!(md).unwrap();

        writeln!(md, "[taxonomies]").unwrap();

        if !self.tags.is_empty() {
            let mut tags = self
                .tags
                .iter()
                .map(|tag| format!("{tag:#?}"))
                .collect::<Vec<String>>();

            tags.sort();

            writeln!(md, "tag = [{tags}]", tags = tags.join(", ")).unwrap();
        }

        if let Some(ref via) = self.via {
            writeln!(md, "via = [{via}]", via = format!("{via:#?}")).unwrap();
        }
        writeln!(md, "+++").unwrap();
        writeln!(md).unwrap();

        let text = format!("{text}\n\n", text = self.text);
        let (before, after) = text.split_once("\n\n").unwrap();

        writeln!(md, "{before}", before = before.trim()).unwrap();
        writeln!(md).unwrap();

        writeln!(md, "<!-- more -->").unwrap();
        writeln!(md).unwrap();

        writeln!(md, "{after}", after = after.trim()).unwrap();

        drop(md);

        println!("Confirm page content:");
        let mut resp = String::new();
        io::stdin().read_line(&mut resp).unwrap();

        Command::new("git")
            .arg("add")
            .arg(&page_path)
            .current_dir(&zola_path)
            .output()
            .unwrap();

        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(&self.subject)
            .current_dir(&zola_path)
            .output()
            .unwrap();
    }

    pub fn confirm(&mut self) {
        dbg!(&self);

        self.subject = ReadLine::new("Title").default(self.subject.clone()).get();

        let mut tags = self
            .tags
            .iter()
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<String>>();

        tags.sort();

        let tags = ReadLine::new("Tags").default(tags.join(" ")).get();

        self.tags = tags
            .split(" ")
            .map(|tag| tag.trim_start_matches("#").to_owned())
            .collect();
    }
}

#[derive(Debug, Deserialize)]
struct MastodonNote {
    #[serde(rename = "attributedTo")]
    pub(crate) attributed_to: Option<String>,

    pub(crate) content: String,
}

#[derive(Debug, Deserialize)]
struct MastodonUser {
    pub(crate) name: String,
}

fn slug_from_title(title: &str) -> String {
    lazy_static! {
        static ref NON_WORD_CHARS: Regex = Regex::new(r#"\W+"#).unwrap();
        static ref TRAILING_HYPHEN: Regex = Regex::new(r#"-$"#).unwrap();
    }

    let title = NON_WORD_CHARS.replace_all(&title, "-").to_string();
    let title = TRAILING_HYPHEN.replace(&title, "").to_string();
    let title = title.to_ascii_lowercase();
    title
}
