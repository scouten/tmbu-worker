use imap::types::Fetch;

pub(crate) fn process_message(message: &Fetch) {
    // Extract the message's body.
    let body = message.body().expect("Message did not have a body!");
    let body = std::str::from_utf8(body)
        .expect("Message was not valid utf-8")
        .to_string();

    println!("{body}");

    if true {
        panic!("One is enough for now ...");
    }
}
