use std::{
    collections::HashMap,
    io::{self, Write},
};

pub(crate) struct ReadLine {
    prompt: String,
    default: Option<String>,
    shortcuts: HashMap<String, String>,
    validate: Box<dyn Fn(&str) -> bool>,
}

impl ReadLine {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_owned(),
            default: None,
            shortcuts: HashMap::new(),
            validate: Box::new(default_validate),
        }
    }

    pub fn default(mut self, default: String) -> Self {
        self.default = Some(default);
        self
    }

    // pub fn default_opt(mut self, default: Option<String>) -> Self {
    //     self.default = default;
    //     self
    // }

    // pub fn shortcuts(mut self, shortcuts: HashMap<String, String>) -> Self {
    //     self.shortcuts = shortcuts;
    //     self
    // }

    // pub fn validate<V: Fn(&str) -> bool + 'static>(mut self, validate: V) -> Self
    // {     self.validate = Box::new(validate);
    //     self
    // }

    pub fn get(self) -> String {
        loop {
            print!(
                "\n{prompt}{default}: ",
                prompt = self.prompt,
                default = self
                    .default
                    .as_ref()
                    .map(|d| format!(" [{d}]"))
                    .unwrap_or_default()
            );
            io::stdout().flush().unwrap();

            let mut resp = String::new();
            io::stdin()
                .read_line(&mut resp)
                .expect("Failed to read line");

            resp = resp.trim().to_owned();

            if resp.is_empty() {
                if let Some(default) = self.default.as_ref() {
                    resp = default.to_owned();
                }
            }

            if let Some(repl) = self.shortcuts.get(&resp) {
                resp = repl.to_owned();
            }

            if self.validate.as_ref()(&resp) {
                return resp;
            }
        }
    }
}

fn default_validate(s: &str) -> bool {
    !s.is_empty()
}
