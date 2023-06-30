use std::env;

fn main() {
    println!("Hello, world!");

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

    // We want to fetch the first email in the INBOX mailbox.
    imap_session.select("INBOX").unwrap();

    // Fetch message number 1 in this mailbox, along with its RFC822 field.
    // RFC 822 dictates the format of the body of e-mails.
    let messages = imap_session.fetch("1", "RFC822").unwrap();
    let message = if let Some(m) = messages.iter().next() {
        m
    } else {
        eprintln!("Mailbox empty");
        return;
    };

    // Extract the message's body.
    let body = message.body().expect("Message did not have a body!");
    let body = std::str::from_utf8(body)
        .expect("Message was not valid utf-8")
        .to_string();

    println!("{body}");

    // Be nice to the server and log out.
    imap_session.logout().unwrap();
}
