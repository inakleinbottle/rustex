use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child as ChildProcess, ChildStdout, Command};
use std::sync::Arc;

use failure::{err_msg, Error};

use outparse::{parse_log, BuildReport};

use crate::config::Config;

#[derive(Debug)]
pub enum JobStatus {
    Pending,
    Active,
    Success,
    Failed,
    NeedsRebuild,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use JobStatus::*;
        match self {
            Active => write!(f, "Active"),
            Pending => write!(f, "pending"),
            Success => write!(f, "succeeded"),
            Failed => write!(f, "failed"),
            NeedsRebuild => write!(f, "needs rebuilding"),
        }
    }
}

#[derive(Debug)]
pub struct Job {
    config: Arc<Config>,
    pub jobname: OsString,
    command: Command,
    child: Option<ChildProcess>,
    pub run_count: u8,
    pub report: Option<BuildReport>,
    pub status: JobStatus,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.jobname.to_string_lossy(), self.status)?;
        if let Some(ref report) = self.report {
            write!(f, ": {}", report)?;
        }
        Ok(())
    }
}

impl Job {
    pub fn new(config: Arc<Config>, path: &Path) -> Job {
        let mut command = config.get_command();
        command.arg(&path);
        Job {
            config,
            jobname: path.file_stem().unwrap().to_owned(),
            command,
            child: None,
            run_count: 0,
            report: None,
            status: JobStatus::Pending,
        }
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
        } else if report.missing_references > 0 && self.run_count == 1 {
            self.spawn().expect("Could not spawn process");
            false
        } else {
            self.status = JobStatus::Success;
            true
        }
    }

    pub fn poll(&mut self) -> bool {
        match self.status {
            JobStatus::Pending => self.poll_pending(),
            JobStatus::Active => self.poll_active(),
            _ => false
        }
    }

    fn poll_active(&mut self) -> bool {
        let child = match self.child {
            Some(ref mut c) => c,
            None => return false,
        };
        match child.try_wait() {
            Ok(Some(r)) => self.check_build_log(r.success()),
            Ok(None) => false,
            Err(_) => {
                self.status = JobStatus::Failed;
                false
            }
        }
    }

    fn poll_pending(&mut self) -> bool {
        if let Err(e) = self.spawn() {
            self.status = JobStatus::Failed;
        }
        false
    }

    pub fn spawn(&mut self) -> Result<(), Error> {
        self.child = Some(self.command.spawn()?);
        self.status = JobStatus::Active;
        self.run_count += 1;
        Ok(())
    }

    pub fn kill(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
    }

    pub fn get_report(&self) -> Result<&BuildReport, Error> {
        if let Some(report) = self.report.as_ref() {
            Ok(report)
        } else {
            Err(err_msg("Report has not be set"))
        }
    }

    pub fn cleanup(&mut self) -> Result<(), Error> {
        let dir = match &self.config.build_directory {
            Some(d) => PathBuf::from(d),
            None => PathBuf::from("."),
        };
        let name = self.jobname.to_string_lossy();
        for f in dir.read_dir()?.map(|f| f.unwrap().path()) {
            if let Some(fname) = f.file_name() {
                if fname.to_string_lossy().starts_with(name.as_ref()) {
                    let ext = f.extension().unwrap();
                    if ext == "tex" || ext == "pdf" {
                        continue;
                    }
                    fs::remove_file(f)?;
                }
            }
        }

        Ok(())
    }
}
