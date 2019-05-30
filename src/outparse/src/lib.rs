use std::collections::HashMap;
use std::fmt;
use std::error::Error;

use regex::{Regex, Captures};
use lazy_static::lazy_static;
use serde::Serialize;

lazy_static! {
    static ref ERROR: Regex = Regex::new(
        r#"^(?:! ((?:La|pdf)TeX|Package|Class)(?: (\w+))? [eE]rror(?: \(([\\]?\w+)\))?: (.*)|! (.*))"#
    ).unwrap();
    
    static ref WARNING: Regex = Regex::new(
        r#"^((?:La|pdf)TeX|Package|Class)(?: (\w+))? [wW]arning(?: \(([\\]?\w+)\))?: (.*)"#
    ).unwrap();

    static ref INFO: Regex = Regex::new(
         r#"^((?:La|pdf)TeX|Package|Class)(?: (\w+))? [iI]nfo(?: \(([\\]?\w+)\))?: (.*)"#
    ).unwrap();

    static ref BADBOX: Regex = Regex::new(
        r#"^(Over|Under)full \\([hv])box \((?:badness (\d+)|(\d+(?:\.\d+)?pt) too \w+)\) (?:(?:(?:in paragraph|in alignment|detected) (?:at lines (\d+)--(\d+)|at line (\d+)))|(?:has occurred while [\\]output is active [\[][\]]))"#
    ).unwrap();
}

#[derive(Debug, Serialize)]
pub struct MessageInfo {
    full: String,
    details: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub enum Message {
    Error(MessageInfo),
    Warning(MessageInfo),
    Badbox(MessageInfo),
    Info(MessageInfo),
}

/*
impl fmt::Display for Message {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n", self.0.full)
    }
}
*/



#[derive(Debug, Serialize)]
pub struct BuildReport {
    errors: usize,
    warnings: usize,
    badboxes: usize,
    info: usize,
    messages: Vec<Message>,
}

impl BuildReport {

    fn new() -> BuildReport {
        BuildReport {
            messages: Vec::new(),
            errors: 0,
            warnings: 0,
            badboxes: 0,
            info: 0
        }
    }

    fn parse(&mut self, line: &str) {
        if let Some(m) = INFO.captures(&line) {
            self.info += 1;
            self.messages.push(BuildReport::process_info(m));
        } else if let Some(m) = BADBOX.captures(&line) {
            self.badboxes += 1;
            self.messages.push(BuildReport::process_badbox(m));
        } else if let Some(m) = WARNING.captures(&line) {
            self.warnings += 1;
            self.messages.push(BuildReport::process_warning(m));
        } else if let Some(m) = ERROR.captures(&line) {
            self.errors += 1;
            self.messages.push(BuildReport::process_error(m));
        }
    }


    /// Parse a log file to generate a build report.
    pub fn parse_log<T: AsRef<str>>(buf: &[T]) 
            -> Result<BuildReport, &'static str> {
        let mut report = BuildReport::new();



        
        Ok(report)
    }

    fn process_generic(m: Captures) -> MessageInfo {
        let mut info = MessageInfo {
            full: m.get(0).unwrap().as_str().to_owned(),
            details: HashMap::new(),
        };

        // 0 - Whole match
        // 1 - Type ((?:La|pdf)TeX|Package|Class)
        // 2 - Package or Class name (\w*)?
        // 3 - extra?
        // 4 - message (.*)
        
        let type_name = m.get(1).unwrap().as_str();
        info.details.insert(
            String::from("type"),
            type_name.to_owned()
        );
        if let Some(name) = m.get(2) {
            let key = match type_name {
                "Package" => String::from("package"),
                "Class"   => String::from("class"),
                _         => String::from("component"),
            };
            info.details.insert(
                key,
                name.as_str().to_owned()
            );
        }

        if let Some(extra) = m.get(3) {
            info.details.insert(
                String::from("extra"),
                extra.as_str().to_owned()
            );
        }

        info.details.insert(
            String::from("message"),
            m.get(4).unwrap().as_str().to_owned()
        );

        info
    }

    fn process_info(m: Captures) -> Message {
        let info = BuildReport::process_generic(m);
        Message::Info(info)
    }

    fn process_badbox(m: Captures) -> Message {
        let mut info = MessageInfo {
            full: m.get(0).unwrap().as_str().to_owned(),
            details: HashMap::new(), 
        };

        // Regex match groups
        // 0 - Whole match
        // 1 - type (Over|Under)
        // 2 - direction ([hv])
        // 3 - underfull box badness (badness (\d+))?
        // 4 - overfull box size (\d+(\.\d+)?pt too \w+)?
        // 5 - Multi-line start line (at lines (\d+)--)?
        // 6 - Multi-line end line (--(\d+))?
        // 7 - Single line (at line (\d+))?
        
        let box_type = m.get(1).unwrap().as_str();
        let direction = m.get(2).unwrap().as_str();
        info.details.insert(
            String::from("type"),
            box_type.to_owned()
        );
        info.details.insert(
            String::from("direction"),
            direction.to_owned()
        );

        if box_type == "Over" {
            let over_by = m.get(4).unwrap().as_str();
            info.details.insert(
                String::from("by"),
                over_by.to_owned()
            );
        } else if box_type == "Under" {
            let badness = m.get(3).unwrap().as_str();
            info.details.insert(
                String::from("by"),
                badness.to_owned()
            );
        }

        if let Some(line) = m.get(7) { 
            // single line
            info.details.insert(
                String::from("line"),
                line.as_str().to_owned()
            );
        } else if let Some(start) = m.get(5) {
            info.details.insert(
                String::from("start_line"),
                start.as_str().to_owned()
            );
            info.details.insert(
                String::from("end_line"),
                m.get(6).unwrap().as_str().to_owned()
            );
        }

        Message::Badbox(info)
    }

    fn process_warning(m: Captures) -> Message {
        let info = BuildReport::process_generic(m);
        Message::Warning(info)
    }

    fn process_error(m: Captures) -> Message {
        if let Some(message) = m.get(5) {
            let mut info = MessageInfo {
                full: m.get(0).unwrap().as_str().to_owned(),
                details: HashMap::new()
            };

            info.details.insert(
                String::from("message"),
                message.as_str().to_owned()
            );
            Message::Error(info)
        } else {
            let info = BuildReport::process_generic(m);
            Message::Error(info)
        }
    }

}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_underfull_vbox_while_output_active() {
        let line = "Underfull \\vbox (badness 1234) has occurred while \\output is active []";
        let mut report = BuildReport::new();
        report.parse(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);
        
    }

    #[test]
    fn test_underfull_vbox_detected_at() {
        let line = "Underfull \\vbox (badness 10000) detected at line 19";
        let mut report = BuildReport::new();
        report.parse(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);

    }

    #[test]
    fn test_underfull_hbox_at_lines() {
        let line = "Underfull \\hbox (badness 1234) in paragraph at lines 9--10";
        let mut report = BuildReport::new();
        report.parse(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);
        
    }

    #[test]
    fn test_overfull_vbox_while_output_active() {
        let line = "Overfull \\vbox (19.05511pt too high) has occurred while \\output is active []";

        let mut report = BuildReport::new();

        report.parse(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);


    }

    #[test]
    fn test_overfull_hbox_on_line() {
        let line = "Overfull \\hbox (54.95697pt too wide) in paragraph at lines 397--397";

        let mut report = BuildReport::new();

        report.parse(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);
    }


    #[test]
    fn test_package_not_found_error() {
        let line = "! LaTeX Error: File `foobar.sty' not found.";

        let mut report = BuildReport::new();

        report.parse(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_undefined_control_sequence_tex_error() {
        let line = "! Undefined control sequence.";

        let mut report = BuildReport::new();

        report.parse(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_too_many_braces_tex_error() {
        let line = "! Too many }'s.";
        let mut report = BuildReport::new();
        report.parse(&line);
        
        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_missing_math_mod_text_error() {
        let line = "! Missing $ inserted";
        let mut report = BuildReport::new();
        report.parse(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_package_error() {
        let line = "! Package babel Error: Unknown option `latin'. Either you misspelled it";
        let mut report = BuildReport::new();
        report.parse(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_pdftex_error() {
        let line = "! pdfTeX error (\\pdfsetmatrix): Unrecognized format..";
        let mut report = BuildReport::new();
        report.parse(&line);

        assert_eq!(report.errors, 1);
    }


    #[test]
    fn test_class_error() {
        let line = "! Class article Error: Unrecognized argument for \\macro.";
        let mut report = BuildReport::new();
        report.parse(&line);

        assert_eq!(report.errors, 1);

    }

}
