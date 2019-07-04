use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::iter::Iterator;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::ops::Deref;
use std::rc::Rc;

use failure::{err_msg, Error as E, bail};



use outparse::{parse_log, BuildReport};

use crate::config::Config;
use crate::engine::get_extension_for_engine;
use crate::jobs::{Job, JobStatus};
use crate::report::RunnerReport;


#[derive(Debug)]
pub enum ReportFormat {
    Human,
    Json,
}


pub struct Runner {
    config: Arc<Config>,

    abort: Arc<AtomicBool>,

    pending: VecDeque<Job>,

    active: VecDeque<Job>,
    completed: Vec<Job>,
    failed: Vec<Job>,

}

impl Runner {

    pub fn new<P: AsRef<Path>>(
        config: Arc<Config>, 
        jobs: &[P]
    )-> Runner {

        let pending = jobs.iter().map(|p| {
            Job::new(config.clone(), p.as_ref())
        }).collect();

        let active = VecDeque::with_capacity(config.max_jobs);
        Runner {
            config,
            abort: Arc::new(AtomicBool::new(false)),
            pending,
            active,
            completed: Vec::new(),
            failed: Vec::new(),

        }
    }

    pub fn submit(&mut self, path: &Path) -> Result<(), E> {
        if !path.exists() {
            bail!("The file {} does not exist", path.display())
        }
        let job = Job::new(self.config.clone(), path);
        self.pending.push_back(job);
        Ok(())
    }

    fn push_next_job(&mut self) {
        if let Some(mut job) = self.pending.pop_front() {
            job.spawn().expect("Cannot launch new job");
            self.active.push_back(job);
        }
    }

    pub fn process_till_next_complete(&mut self) -> Option<&Job> {

        if self.active.is_empty() && !self.pending.is_empty() {
            (0..self.active.capacity()).for_each(|_| self.push_next_job());
        }

        while !self.active.is_empty() {
            if let Some(i) = self.active.iter_mut().position(|j| j.poll()) {
                let job = self.active.remove(i).unwrap();
                self.completed.push(job);

                self.push_next_job();
                return Some(self.completed.last().unwrap())
            }
        }


        None
    }

    fn kill(&mut self) {
        self.abort.store(true, Ordering::Release);
        self.active.iter_mut().for_each(|j| j.kill());
        self.pending.clear();
    }

    pub fn do_cleanup(&mut self) -> Result<(), E> {
        if !self.config.clean_build {
            return Ok(());
        }
        for job in self.completed.iter_mut() {
            job.cleanup()?;
        }
        Ok(())
    }

    pub fn build_report(&self) -> Result<RunnerReport, E> {
        use JobStatus::*;
        let mut report = RunnerReport::new();
        report.num_files = self.completed.len();
        for job in &self.completed {
            match &job.status {
                Success => report.success += 1,
                Failed => report.fail += 1,
                _ => return Err(err_msg("Job was not completed.")),
            }
        }
        Ok(report)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> Arc<Config> {
        Arc::new(Config::default())
    }

    #[test]
    fn test_build_with_pdflatex() {
        let config = make_config();
        let path = PathBuf::from("test.tex");
        let pths = [&path];
        let mut runner = Runner::new(config, &pths);
        
        
        let job = runner.process_till_next_complete().unwrap();
        
        let report = job.get_report().unwrap();
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.badboxes, 0);
    }

}
