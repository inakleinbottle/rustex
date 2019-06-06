
use std::ffi::{OsString};
use std::path::{Path};
use std::process::{Command, Child as ChildProcess, ChildStdout};

use failure::{Error, err_msg};

use outparse::{BuildReport, parse_log};

use crate::config::Config;

#[derive(Debug)]
pub enum JobStatus {
    Pending,
    Success,
    Failed,
    NeedsRebuild,
}

#[derive(Debug)]
pub struct Job {
    pub jobname: OsString,
    command: Command,
    child: Option<ChildProcess>,
    pub run_count: u8,
    pub report: Option<BuildReport>,
    pub status: JobStatus,
}

impl Job {

    pub fn new(config: &Config, path: &Path) -> Result<Job, Error> {
        let mut command = config.get_command();
        command.arg(&path);
        let mut job = Job {
            jobname: path.file_name().unwrap().to_owned(),
            command,
            child: None, 
            run_count: 0,
            report: None,
            status: JobStatus::Pending,
        };
        job.spawn()?;
        Ok(job)
    }

    fn stdout(&mut self) -> Option<ChildStdout> {
        if let Some(ref mut child) = self.child {
            child.stdout.take()
        } else {
            None
        }
    }

    fn check_build_log(&mut self, exit_code_success: bool) -> bool {
        let stdout = self.stdout().unwrap();
        self.report = Some(parse_log(stdout));
        let report = self.report.as_ref().unwrap();

        if report.errors > 0 || !exit_code_success {
            self.status = JobStatus::Failed;
            true
        } else if report.warnings > 0 && self.run_count == 1 {
            self.spawn().expect("Could not spawn process");
            false
        } else {
            self.status = JobStatus::Success;
            true
        }
    }

    pub fn poll(&mut self) -> bool {
        let child = match self.child {
            Some(ref mut c) => c,
            None => return false
        };
        match child.try_wait() {
            Ok(Some(r)) => self.check_build_log(r.success()),
            Ok(None)    => false,
            Err(e)      => {
                self.status = JobStatus::Failed;
                true
            }

        }
    }

    pub fn spawn(&mut self) -> Result<(), Error> {
        self.child = Some(self.command.spawn()?);
        self.run_count += 1;
        Ok(())
    }

    pub fn get_report(&mut self) -> Result<&BuildReport, Error> {
        if let Some(report) = self.report.as_ref() {
            Ok(report)
        } else {
            Err(err_msg("Report has not be set"))
        }
    }

    pub fn cleanup(&mut self) -> Result<(), Error> {

        Ok(())
    }

}

