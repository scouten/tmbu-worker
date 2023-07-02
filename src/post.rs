use std::collections::HashSet;

use chrono::{offset::FixedOffset, DateTime};
use serde::Deserialize;

use crate::message::Message;

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
            Some(ref link) => link,
            None => {
                return;
            }
        };

        let client = reqwest::blocking::Client::builder().build().unwrap();

        let note: MastodonNote = match client
            .get(link)
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

        let user: MastodonUser = match client
            .get(&note.attributed_to)
            .header("Accept", "application/activity+json")
            .send()
            .and_then(|r| r.json())
        {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!(
                    "Unable to follow user link {link}\n\n{e}\n",
                    link = note.attributed_to
                );
                return;
            }
        };

        // TO DO: parse out link?

        // OK, this is definitely a Mastodon post link.
        // Update pending Zola post accordingly.
        self.via = Some("Mastodon".to_owned());

        let mut text = self.text.clone();
        text = text.replace(link, "");
        text = text.trim().to_owned();

        self.text = format!(
            "via [{user_name}]({user_link}): {user_comment}\n\n<!-- more -->\n\n{text}",
            user_name = user.name,
            user_link = note.attributed_to,
            user_comment = note.content
        );

        self.link = None;

        // dbg!(&note);
        // dbg!(&user);
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // TEMPORARY while building
struct MastodonNote {
    pub(crate) url: String,

    #[serde(rename = "attributedTo")]
    pub(crate) attributed_to: String,

    pub(crate) content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // TEMPORARY while building
struct MastodonUser {
    pub(crate) name: String,
}
