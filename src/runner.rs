use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Child as ChildProcess, Stdio};
use serde::{Serialize, Deserialize};

use failure::{Error as E, err_msg};
use structopt::{StructOpt};

use outparse::{parse_log, BuildReport};
use crate::engine::get_extension_for_engine;
use crate::report::RunnerReport;
use crate::jobs::{Job, JobStatus};
use crate::config::Config;


#[derive(Debug)]
pub enum ReportFormat {
    Human,
    Json,
}

pub struct Runner<'cfg> {
    config: &'cfg Config,

    active: VecDeque<Job>,
    completed: Vec<Job>

}

impl<'cfg> Runner<'cfg> {

    pub fn new(config: &'cfg Config) -> Runner {
        Runner {
            config,
            active: VecDeque::new(),
            completed: Vec::new(),
        }
    }

    fn submit(&mut self, path: &Path) -> Result<(), E> {
        let job = Job::new(&self.config, path)?;
        self.active.push_back(job);
        Ok(())
    }
    
    fn process_submissions(&mut self) -> Result<RunnerReport, E> {
        while let Some(mut job) = self.active.pop_front() {
            if job.poll() {
                self.completed.push(job); 
            } else {
                self.active.push_back(job);
            }
        }
        self.do_cleanup()?;

        self.build_report()
    }

    fn do_cleanup(&mut self) -> Result<(), E> {
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
                Failure => report.fail += 1,
                _ => return Err(err_msg("Job was not completed."))
            }
        }
        Ok(report)
    }

    pub fn run(&mut self) -> Result<RunnerReport, E> {
        for p in self.config.paths() {
            self.submit(&p)?
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

    fn make_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_build_with_pdflatex() {
        let config = make_config();
        let path = PathBuf::from("test.tex");
        let mut runner = Runner::new(&config);
        runner.submit(&path).expect(
            "An error occured whilst submitting task"
        );

        assert_eq!(runner.active.len(), 1);

        runner.process_submissions().expect(
            "An error occured whilst processing task"
        );

        assert_eq!(runner.active.len(), 0);
        assert_eq!(runner.completed.len(), 1);

        let job = runner.completed.get(0).unwrap();
        let report = job.report.as_ref().unwrap();
        assert_eq!(report.errors, 0);
        assert_eq!(report.warnings, 0);
        assert_eq!(report.badboxes, 0);

    }





}