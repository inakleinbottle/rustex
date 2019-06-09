use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::slice::Iter;

use failure::{err_msg, Error as E};
use indicatif::ProgressBar;
use structopt::StructOpt;

use outparse::{BuildReport, Message};

use crate::config::Config;
use crate::jobs::Job;
use crate::report::RunnerReport;
use crate::runner::{ReportIF, Runner};

struct CLIReporter {
    pb: ProgressBar,
    config: Rc<Config>,
}

impl CLIReporter {
    fn print_message(&self, message: &Message) {
        if let Some(inner) = message.as_ref() {
            self.pb.println(inner.full.as_str());
        } else {
            match message {
                Message::MissingReference { label } => {
                    self.pb.println(format!("Missing label: {}", label));
                }
                Message::MissingCitation { label } => {
                    self.pb.println(format!("Missing citation: {}", label));
                }
                _ => {}
            };
        }
    }
}

impl ReportIF for CLIReporter {
    fn finish(&self, report: &RunnerReport) {
        let message = format!("{}", report);
        self.pb.finish_with_message(&message);
    }

    fn report_completed(&self, job: &Job) {
        self.pb.inc(1);
        self.pb.println(format!("{}", job));
        if self.config.verbose {
            if let Some(ref report) = job.report {
                for message in &report.messages {
                    self.print_message(&message)
                }
            }
        }
    }
}

impl CLIReporter {
    fn new(config: Rc<Config>, num_files: usize) -> CLIReporter {
        CLIReporter {
            pb: ProgressBar::new(num_files as u64),
            config,
        }
    }
}

/// LaTeX file build utility.
///
/// This is essentially a wrapper around the LaTeX executables
/// that adds intelligent building and simplified build reports.
/// Also supports running auxiliary programs such as BibTeX,
/// Biber, and Makeindex. The builder will generate an appropriate
/// build order for in this case.
///
/// The builder supports multiple input file build jobs, and the
/// jobs are executed asyncronously, by making non-blocking calls
/// to the underlying LaTeX engine.
#[derive(StructOpt)]
pub struct CliOptions {
    #[structopt(flatten)]
    pub config: Config,

    /*
    /// Maximum number of build attempts.
    ///
    /// Maximum number of attempts to build the document
    /// and remove any warnings generated by unresolved
    /// references. Default value: 2.
    #[structopt(long="max-rebuilds", default_value="2")]
    max_rebuilds: u8,
    */

    /*
    /// Force LaTeX engine to execute twice.
    ///
    /// This does not apply if the build fails
    /// due to an error.
    #[structopt(long="force-two-runs")]
    force_two_builds: bool,
    */
    /// Files to build
    ///
    /// THe files to attempt to build in this run of
    /// LaTeX.
    #[structopt(name = "files", parse(from_os_str))]
    pub files: Vec<PathBuf>,
}

impl Default for CliOptions {
    fn default() -> CliOptions {
        CliOptions {
            config: Config::default(),
            files: Vec::new(),
        }
    }
}

pub fn run() -> Result<(), E> {
    let CliOptions { config, files } = CliOptions::from_args();
    let conf = Rc::new(config);

    // do the setup for verbosity etc.
    let inner = CLIReporter::new(conf.clone(), files.len());
    let reporter = Box::new(inner);
    let mut runner = Runner::new(conf.clone(), reporter);

    let _report = runner.run(&files)?;
    Ok(())
}
