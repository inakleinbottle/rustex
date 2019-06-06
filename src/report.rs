use std::collections::HashMap;
use std::fmt;
use std::ffi::OsString;

use failure::{Error, err_msg};

use outparse::BuildReport;

pub type ReportMap = HashMap<OsString, BuildReport>;

#[derive(Debug)]
pub struct RunnerReport {
    pub num_files: usize,
    pub success: usize,
    pub fail: usize,
    pub build_reports: ReportMap,
}


impl fmt::Display for RunnerReport {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Build statistics: {} jobs, {} succeeded, {} failed.", 
            self.num_files, self.success, self.fail
        )
    }

}

impl RunnerReport {

    pub(crate) fn new() -> RunnerReport {
        RunnerReport {
            num_files: 0,
            success: 0,
            fail: 0,
            build_reports: ReportMap::new()
        }
    }
}