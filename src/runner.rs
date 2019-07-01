use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::iter::Iterator;
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

    pending: Vec<Job>,
    active: VecDeque<Job>,
    completed: Vec<Job>,
    failed: Vec<Job>

}

impl Runner {
    pub fn new<P: AsRef<Path>>(
        config: Arc<Config>, 
        reporter: Arc<dyn ReportIF + Send + Sync>,
        jobs: &[P]
    )-> Runner {
        let pending: Vec<Job> = jobs.iter().map(|p| {
            Job::new(config.clone(), p.as_ref())
        }).collect();
        Runner {
            config,
            reporter,
            abort: Arc::new(AtomicBool::new(false)),
            pending,
            active: VecDeque::new(),
            completed: Vec::new(),
            failed: Vec::new()
        }
    }

    fn submit(&mut self, path: &Path) -> Result<(), E> {
        if !path.exists() {
            bail!("The file {} does not exist", path.display())
        }
        let job = Job::new(self.config.clone(), path);
        self.pending.push(job);
        Ok(())
    }

    fn launch_pending_jobs(&mut self) {
        while let Some(mut job) = self.pending.pop() {
            job.spawn().expect("Could not spawn task");
            self.active.push_back(job);
        }
    }

    fn process_till_next_complete(&mut self) -> Option<BuildReport> {
        if !self.pending.is_empty() {
                self.launch_pending_jobs();
        }
        
        while let Some(mut job) = self.active.pop_front() {
            
            if job.poll() {
                self.completed.push(job);
                let j = self.completed.last().unwrap();
                return Some(j.report.as_ref().unwrap().clone())
            } else {
                self.active.push_back(job);
            }
        }
        None
    }

    fn process_submissions(&mut self) -> Result<RunnerReport, E> {
        while let Some(mut job) = self.active.pop_front() {
            if self.abort.load(Ordering::Relaxed) {
                job.kill();
                self.kill();
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

impl Iterator for Runner {
    type Item = BuildReport;

    fn next(&mut self) -> Option<Self::Item> {
        self.process_till_next_complete()
    }
}

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
        let mut runner = Runner::new(config, reporter, &[&path]);

        assert_eq!(runner.pending.len(), 1);

        runner.launch_pending_jobs();
        assert_eq!(runner.pending.len(), 0);
        assert_eq!(runner.active.len(), 1);

        let report = runner.process_till_next_complete();
        
        assert_eq!(runner.active.len(), 0);
        assert_eq!(runner.completed.len(), 1);

        let job = runner.completed.get(0).unwrap();
        let report = job.report.as_ref().unwrap();
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.badboxes, 0);
    }

}
