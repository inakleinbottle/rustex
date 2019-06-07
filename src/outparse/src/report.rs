use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct MessageInfo {
    pub full: String,
    pub details: HashMap<String, String>,
    pub context_lines: Vec<String>,
}

impl MessageInfo {
    fn get_component_name<'a>(&'a self) -> Option<&'a str> {
        if self.details.contains_key("component") {
            Some(&self.details.get("component").unwrap())
        } else if self.details.contains_key("package") {
            Some(&self.details.get("package").unwrap())
        } else if self.details.contains_key("class") {
            Some(&self.details.get("class").unwrap())
        } else {
            None
        }
    }

    fn add_context(&mut self, line: String) {
        self.context_lines.push(line);
    }

    fn extend_message(&mut self, message: &str) {
        if let Some(current) = self.details.get_mut("message") {
            current.push_str(message);
        } else {
            self.details
                .insert(String::from("message"), message.to_owned());
        }
    }
}

#[derive(Debug, Serialize)]
pub enum Message {
    Error(MessageInfo),
    Warning(MessageInfo),
    Badbox(MessageInfo),
    Info(MessageInfo),
}

impl Message {
    pub(crate) fn get_component_name<'a>(&'a self) -> Option<&'a str> {
        use Message::*;
        match self {
            Error(ref inner) => inner.get_component_name(),
            Warning(ref inner) => inner.get_component_name(),
            Info(ref inner) => inner.get_component_name(),
            Badbox(_) => None,
        }
    }

    pub(crate) fn extend_message(&mut self, message: &str) {
        use Message::*;
        match self {
            Error(ref mut inner) => inner.extend_message(message),
            Warning(ref mut inner) => inner.extend_message(message),
            Info(ref mut inner) => inner.extend_message(message),
            Badbox(_) => return,
        }
    }

    pub(crate) fn add_context(&mut self, line: String) {
        use Message::*;
        match self {
            Error(ref mut inner) => inner.add_context(line),
            Warning(ref mut inner) => inner.add_context(line),
            Info(ref mut inner) => inner.add_context(line),
            Badbox(ref mut inner) => inner.add_context(line),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BuildReport {
    pub errors: usize,
    pub warnings: usize,
    pub badboxes: usize,
    pub info: usize,
    pub messages: Vec<Message>,
}

impl BuildReport {
    pub(crate) fn new() -> BuildReport {
        BuildReport {
            messages: Vec::new(),
            errors: 0,
            warnings: 0,
            badboxes: 0,
            info: 0,
        }
    }
}

impl fmt::Display for BuildReport {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Errors: {}, Warnings: {}, Badboxes: {}",
            self.errors,
            self.warnings,
            self.badboxes,
        )
    }
}