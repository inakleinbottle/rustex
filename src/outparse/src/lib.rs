use std::collections::HashMap;
use std::fmt;
use std::error::Error;
use std::io::prelude::*;
use std::io::{self, BufReader};

use regex::{Regex, Captures};
use lazy_static::lazy_static;
use serde::Serialize;

pub mod report;
pub use report::*;

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









struct LogParser<'a, B: BufRead> {
    report: &'a mut BuildReport,
    reader: B,
    last_message: Option<&'a mut Message>,
    lineno: usize,
    collect_remaining: Option<usize>,
    context_lines: usize
}


impl<'a, B: BufRead> LogParser<'a, B> {


    fn next_line(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(_) => Some(line),
            Err(_) => None
        }
    }

    fn parse_line(&mut self, line: &str) {
        if let Some(m) = INFO.captures(&line) {
            self.process_info(m);
        } else if let Some(m) = BADBOX.captures(&line) {
            self.process_badbox(m);
        } else if let Some(m) = WARNING.captures(&line) {
            self.process_warning(m);
        } else if let Some(m) = ERROR.captures(&line) {
            self.process_error(m);
        }
    }

    fn process_generic(&mut self, m: Captures) -> MessageInfo {
        let mut info = MessageInfo {
            full: m.get(0).unwrap().as_str().to_owned(),
            details: HashMap::new(),
            context_lines: Vec::new(),
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

    fn process_info(&mut self, m: Captures) {
        let info = self.process_generic(m);
        self.report.info += 1;
        self.report.messages.push(Message::Info(info));
    }

    fn process_badbox(&mut self, m: Captures) {
        let mut info = MessageInfo {
            full: m.get(0).unwrap().as_str().to_owned(),
            details: HashMap::new(),
            context_lines: Vec::new(),
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

        self.report.badboxes += 1;
        self.report.messages.push(Message::Badbox(info));
    }

    fn process_warning(&mut self, m: Captures) {
        let info = self.process_generic(m);
        self.report.warnings += 1;
        self.report.messages.push(Message::Warning(info));
    }

    fn process_error(&mut self, m: Captures) {
        if let Some(message) = m.get(5) {
            let mut info = MessageInfo {
                full: m.get(0).unwrap().as_str().to_owned(),
                details: HashMap::new(),
                context_lines: Vec::new(),
            };

            info.details.insert(
                String::from("message"),
                message.as_str().to_owned()
            );
            self.report.errors += 1;
            self.report.messages.push(Message::Error(info))
        } else {
            let info = self.process_generic(m);
            self.report.errors += 1;
            self.report.messages.push(Message::Error(info))
        }
    }

}


impl<'a, B: BufRead> LogParser<'a, B> {

    pub fn new(report: &'a mut BuildReport, reader: B, context_lines: usize) -> LogParser<'a, B> {
        LogParser {
            report,
            reader,
            last_message: None,
            lineno: 0,
            collect_remaining: None,
            context_lines,
        }
    }

    pub fn parse(&mut self) {
        loop {
            let line = match self.next_line() {
                Some(l) => l,
                None => continue
            };

            if let Some(ref mut last) = self.last_message {
                if let Some(cmpt) = last.get_component_name() {
                    let pattern = format!("({}) ", cmpt);
                    if line.starts_with(&pattern) {
                        let message = line.trim_start_matches(&pattern).trim_start();
                        last.extend_message(&message);
                        continue
                    }
                } 
            }

            if let Some(ref mut i) = self.collect_remaining {
                *i -= 1;
                if let Some(ref mut last) = self.last_message {
                    last.add_context(line);
                }
                if *i == 0 {
                    self.collect_remaining = None;
                }
                continue
            }

            self.parse_line(&line);

        }
    }

}

pub fn parse_log<R: Read>(log: R) -> BuildReport {
    let mut reader = BufReader::new(log);
    let mut report = BuildReport::new();

    let mut parser: LogParser<BufReader<R>> = LogParser::new(
        &mut report, reader, 2
    );


    report
}







#[cfg(test)]
mod tests {
    use super::*;

    fn create_parser(line: &str) -> BuildReport {
        let mut cursor = io::Cursor::new(&line);
        let mut reader = BufReader::new(cursor);
        let mut report = BuildReport::new();
        let mut parser = LogParser::new(&mut report, reader, 2);
        parser.parse_line(&line);
        report
    }

    #[test]
    fn test_underfull_vbox_while_output_active() {
        let line = "Underfull \\vbox (badness 1234) has occurred while \\output is active []";
        let report = create_parser(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);
        
    }

    #[test]
    fn test_underfull_vbox_detected_at() {
        let line = "Underfull \\vbox (badness 10000) detected at line 19";
        let report = create_parser(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);

    }

    #[test]
    fn test_underfull_hbox_at_lines() {
        let line = "Underfull \\hbox (badness 1234) in paragraph at lines 9--10";
        let report = create_parser(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);
        
    }

    #[test]
    fn test_overfull_vbox_while_output_active() {
        let line = "Overfull \\vbox (19.05511pt too high) has occurred while \\output is active []";
        let report = create_parser(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);


    }

    #[test]
    fn test_overfull_hbox_on_line() {
        let line = "Overfull \\hbox (54.95697pt too wide) in paragraph at lines 397--397";
        let report = create_parser(&line);

        assert_eq!(report.badboxes, 1);
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.info, 0);
    }


    #[test]
    fn test_package_not_found_error() {
        let line = "! LaTeX Error: File `foobar.sty' not found.";
        let report = create_parser(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_undefined_control_sequence_tex_error() {
        let line = "! Undefined control sequence.";
        let report = create_parser(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_too_many_braces_tex_error() {
        let line = "! Too many }'s.";
        let report = create_parser(&line);
        
        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_missing_math_mod_text_error() {
        let line = "! Missing $ inserted";
        let report = create_parser(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_package_error() {
        let line = "! Package babel Error: Unknown option `latin'. Either you misspelled it";
        let report = create_parser(&line);

        assert_eq!(report.errors, 1);
    }

    #[test]
    fn test_pdftex_error() {
        let line = "! pdfTeX error (\\pdfsetmatrix): Unrecognized format..";
        let report = create_parser(&line);

        assert_eq!(report.errors, 1);
    }


    #[test]
    fn test_class_error() {
        let line = "! Class article Error: Unrecognized argument for \\macro.";
        let report = create_parser(&line);

        assert_eq!(report.errors, 1);

    }

}
