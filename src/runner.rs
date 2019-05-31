use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Child as ChildProcess, Stdio};
use serde::{Serialize, Deserialize}

use failure::{Error as E, err_msg};

use outparse::{parse_log, BuildReport};

pub enum ReportFormat {
    Human,
    Json,
}


#[derive(Debug, Serialize, Deserialize)]
struct LatexRunner {
    engine: String,
    flags: Vec<String>,
    output_dir: Option<PathBuf>

    max_rebuilds: usize,
    force_two_runs: bool,

    clean_build: bool,
    report_fmt: ReportFormat,

    running_processes: VecDeque<(Command, ChildProcess, u8)>,
    build_reports: Vec<BuildReport>
}


impl Default for LatexRunner {

    fn default() -> Self {
        LatexRunner {
            engine: String::from("pdflatex"),
            flags: vec![String::from("-interaction=nonstopmode")],
            output_dir: None;
            max_rebuilds: 2,
            force_two_runs: false,
            clean_build: false,
            report_fmt: ReportFormat::Human,
            running_processes: VecDeque::new(),
            build_reports: Vec::new(),
        }
    }
}

struct BuildError(BuildReport);

impl BuildError {
    fn report(self) -> BuildReport {
        self.0
    }
}

impl LatexRunner {

    fn new_command(&self, path: &Path) -> Command {
        let mut cmd = Command::new(self.engine);
        &self.flags.for_each(|f| cmd.arg(f));
        if let Some(p) = self.output_dir {
            let flag = OsString.from("-output-directory=");
            flag.push(p.as_os_str());
            cmd.arg(&flag);
        }
        cmd.arg(&path);
        cmd.stdout(Stdio::piped())
        cmd.stderr(Stdio::inherit())
        cmd
    }

    fn submit(&mut self, path: &Path) -> Result<(), E> {
        let cmd = self.new_command(&path);
        let child = cmd.spawn()?;
        self.running_processes.push_back((Command, child, 1));
        Ok(())
    }

    fn check_child(&mut self, child: &ChildProcess, repeats: &u8)
            -> Result<Option<(bool, BuildReport)>, BuildError> {
        if let Some(result) = child.try_wait()? {

            let stdout = child.stdout.unwrap();
            let report = parse_log(stdout);

            if report.errors > 0 {
                Err(BuildError(report))
            } else if report.warnings > 0 && *repeats < self.max_rebuilds {
                Ok(Some(true, report))
            } else if *repeats == 1 && self.force_two_runs {
                Ok(Some(true, report))
            } else {
                Ok(Some(false, report))
            }

        } else {
            Ok(None)
        }
    } 

    fn process_submissions(&mut self) -> Result<(), E> {
        while !self.running_processes.isempty() {
            let (cmd, child, repeats) = self.running_processes.pop_front();
            match self.check_child(&child, &repeats) {
                Ok(Some((true, _)) => {
                    let new_child = cmd.spawn()?;
                    self.running_processes.push_back(
                        (cmd, new_child, repeats + 1)
                    );
                },
                Ok(Some((false, report)) => self.build_reports.push(report),
                Ok(None) => self.running_processes.push_back(
                                (cmd, child, repeats)
                            );
                Err(build_error) => {
                    println!("An error occured {}", build_error.report());
                }
            }
        }
        Ok(())
    }
}