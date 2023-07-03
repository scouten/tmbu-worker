use std::env;

mod message;
mod post;

fn main() {
    // Adapted from example at https://github.com/jonhoo/rust-imap/tree/v2.4.1#readme.

    let domain = env::var("TMBU_IMAP_HOST").unwrap();
    let tls = native_tls::TlsConnector::builder().build().unwrap();

    // We pass in the domain twice to check that the server's TLS
    // certificate is valid for the domain we're connecting to.
    let client = imap::connect((domain.clone(), 993), domain, &tls).unwrap();

    // The client we have here is unauthenticated.
    // To do anything useful with the e-mails, we need to log in.
    let username = env::var("TMBU_IMAP_USERNAME").unwrap();
    let password = env::var("TMBU_IMAP_PASSWORD").unwrap();

    let mut imap_session = client.login(username, password).unwrap();

    // Read contents of primary inbox.
    imap_session.select("INBOX").unwrap();

    // Read first 500 message in inbox.
    let messages = imap_session.fetch("1:500", "RFC822").unwrap();

    let count = messages.len();
    println!("Found {count} messages in inbox");

    for imap_message in messages.iter() {
        // Pick apart the important parts of the IMAP message.
        let message = message::Message::from(&imap_message);

        let message = match message {
            Some(message) => message,
            None => {
                let body = imap_message.body().expect("Message did not have a body!");
                let body = std::str::from_utf8(body)
                    .expect("Message was not valid utf-8")
                    .to_string();

                eprintln!("Failed to parse message\n\n{body}\n\n");
                panic!("Failed to parse message");
            }
        };

        // Now turn the parsed message into a pending Zola post.
        let mut post = post::Post::from(message);
        post.update_if_mastodon_link();
        post.add_link_text();
        post.capitalize_tags();
        post.render();

        imap_session
            .store(format!("{}", imap_message.message), "+FLAGS (\\Deleted)")
            .unwrap();

        if true {
            panic!("One is enough for now ...");
        }
    }

    // Be nice to the server and log out.
    imap_session.logout().unwrap();
}
