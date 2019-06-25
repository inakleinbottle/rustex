use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use failure::{err_msg, Error as E, bail};
use indicatif::ProgressBar;
use structopt::StructOpt;


use outparse::{parse_log, BuildReport};

use crate::config::Config;
use crate::engine::get_extension_for_engine;
use crate::jobs::{Job, JobStatus};
use crate::report::RunnerReport;

pub trait ReportIF {
    fn finish(&self, report: &RunnerReport);
    fn report_completed(&self, job: &Job);
    fn send_message(&self, message: &str);
    fn abort(&self);
}


pub struct NoReporter;

impl ReportIF for NoReporter {
    fn finish(&self, report: &RunnerReport) {}
    fn report_completed(&self, job: &Job) {}
    fn send_message(&self, message: &str) {}
    fn abort(&self) {}
}

#[derive(Debug)]
pub enum ReportFormat {
    Human,
    Json,
}

pub struct Runner {
    config: Arc<Config>,
    reporter: Arc<dyn ReportIF + Send + Sync>,

    abort: Arc<AtomicBool>,

    active: VecDeque<Job>,
    completed: Vec<Job>,
}

impl Runner {
    pub fn new(
        config: Arc<Config>, 
        reporter: Arc<dyn ReportIF + Send + Sync>
    )-> Runner {
        Runner {
            config,
            reporter,
            abort: Arc::new(AtomicBool::new(false)),
            active: VecDeque::new(),
            completed: Vec::new(),
        }
    }

    fn submit(&mut self, path: &Path) -> Result<(), E> {
        if !path.exists() {
            bail!("The file {} does not exist", path.display())
        }
        let job = Job::new(self.config.clone(), path)?;
        self.active.push_back(job);
        Ok(())
    }

    fn process_submissions(&mut self) -> Result<RunnerReport, E> {
        while let Some(mut job) = self.active.pop_front() {
            if self.abort.load(Ordering::Relaxed) {
                job.kill();
                self.kill();
            }
            if job.poll() {
                self.reporter.report_completed(&job);
                self.completed.push(job);
            } else {
                self.active.push_back(job);
            }
        }
        self.do_cleanup()?;

        let report = self.build_report()?;
        self.reporter.finish(&report);
        Ok(report)
    }

    fn kill(&mut self) {
        self.active.iter_mut().for_each(|j| j.kill());
        self.reporter.abort();
        exit(1);
    }

    fn do_cleanup(&mut self) -> Result<(), E> {
        if !self.config.clean_build {
            return Ok(());
        }
        for job in self.completed.iter_mut() {
            job.cleanup()?;
        }
        Ok(())
    }

    fn build_report(&self) -> Result<RunnerReport, E> {
        use JobStatus::*;
        let mut report = RunnerReport::new();
        report.num_files = self.completed.len();
        for job in &self.completed {
            let jobname = job.jobname.clone();
            match &job.status {
                Success => report.success += 1,
                Failed => report.fail += 1,
                _ => return Err(err_msg("Job was not completed.")),
            }
        }
        Ok(report)
    }

    pub fn run(&mut self, paths: &Vec<PathBuf>) -> Result<RunnerReport, E> {
        let reporter = self.reporter.clone();
        let abort_var = self.abort.clone();
        ctrlc::set_handler(move || {
            reporter.send_message("Keyboard interupt received");
            abort_var.store(true, Ordering::Relaxed);
        })?;

        for p in paths {
            self.submit(p)?;
        }

        self.process_submissions()
    }
}

/*

#[derive(Debug)]
struct LatexRunner {
    engine: String,
    flags: Vec<String>,
    output_dir: Option<PathBuf>,

    max_rebuilds: u8,
    force_two_runs: bool,

    clean_build: bool,
    report_fmt: ReportFormat,

    active_jobs: VecDeque<Job>,
    completed_jobs: Vec<Job>,
}

impl From<&LatexRunnerConfig> for LatexRunner {

    fn from(config: &LatexRunnerConfig) -> LatexRunner {
        let outdir = match &config.build_directory {
            Some(ostr) => Some(PathBuf::from(ostr)),
            None => None,
        };

        LatexRunner{
            engine: config.engine.clone(),
            flags: config.flags.clone(),
            output_dir: outdir,

            max_rebuilds: config.max_rebuilds,
            force_two_runs: config.force_two_builds,

            clean_build: config.clean_build,

            report_fmt: ReportFormat::Human,

            active_jobs: VecDeque::new(),
            completed_jobs: Vec::new(),
        }
    }

}


impl Default for LatexRunner {

    fn default() -> Self {
        LatexRunner {
            engine: String::from("pdflatex"),
            flags: vec![String::from("-interaction=nonstopmode")],
            output_dir: None,
            max_rebuilds: 1,
            force_two_runs: false,
            clean_build: false,
            report_fmt: ReportFormat::Human,
            active_jobs: VecDeque::new(),
            completed_jobs: Vec::new(),
        }
    }

}



impl LatexRunner {


    fn clean_build_dir(&self) -> Result<(), E> {
        let dir = match &self.output_dir {
            Some(p) => p.to_owned(),
            None    => PathBuf::from(".")
        };
        let ext = get_extension_for_engine(&self.engine);


        Ok(())
    }

    fn do_cleanup(&mut self) -> Result<(), E> {
        if self.clean_build {
            self.clean_build_dir()
        } else {
            Ok(())
        }
    }

    fn build_report(&self) -> Result<RunnerReport, E> {
        use JobStatus::*;
        let mut report = RunnerReport::new();
        report.num_files = self.completed_jobs.len();
        for job in &self.completed_jobs {
            let jobname = job.jobname.clone();
            match &job.status {
                Success => report.success += 1,
                Failure => report.fail += 1,
                _ => return Err(err_msg("Job was not completed."))
            }
        }
        Ok(report)
    }
}

*/

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> Arc<Config> {
        Arc::new(Config::default())
    }

    fn make_reporter() -> Arc<NoReporter> {
        Arc::new(NoReporter {})
    }

    #[test]
    fn test_build_with_pdflatex() {
        let config = make_config();
        let path = PathBuf::from("test.tex");
        let reporter = make_reporter();
        let mut runner = Runner::new(config, reporter);
        runner
            .submit(&path)
            .expect("An error occured whilst submitting task");

        assert_eq!(runner.active.len(), 1);

        runner
            .process_submissions()
            .expect("An error occured whilst processing task");

        assert_eq!(runner.active.len(), 0);
        assert_eq!(runner.completed.len(), 1);

        let job = runner.completed.get(0).unwrap();
        let report = job.report.as_ref().unwrap();
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.badboxes, 0);
    }

}
