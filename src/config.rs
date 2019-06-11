use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::slice::Iter;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Config {
    /// Use verbose mode.
    ///
    /// More output will be generated during the build
    #[structopt(short = "v", long = "verbose")]
    pub verbose: bool,

    /// LaTeX executable to use. (The build engine.)
    ///
    /// Must be an executable on PATH. Default="pdflatex"
    #[structopt(long = "engine", default_value = "pdflatex", parse(from_os_str))]
    pub engine: OsString,

    /// LaTeX flags to use.
    ///
    /// Multiple calls add more flags.
    #[structopt(long = "latex-flag", parse(from_os_str))]
    pub flags: Vec<OsString>,

    /// Directory in which the build occurs.
    ///
    /// Specify a different directory for the output of the
    /// LaTeX build process.
    #[structopt(long = "build-dir", parse(from_os_str))]
    pub build_directory: Option<OsString>,

    /// Clean build directory after build.
    ///
    /// If selected, the auxiliary files, log files, and
    /// other output files (excluding the output of LaTeX)
    /// will be removed from the build directory. Unless
    /// a second build is forced, a second build run will
    /// only be executed if there are unresolved warnings.
    #[structopt(long = "clean")]
    pub clean_build: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            engine: OsString::from("pdflatex"),
            flags: vec![],
            build_directory: None,
            clean_build: false,
            verbose: false,
        }
    }
}

impl Config {
    pub fn get_command(&self) -> Command {
        let mut cmd = Command::new(&self.engine);
        for f in &self.flags {
            cmd.arg(f);
        }
        cmd.arg(OsString::from("-interaction=nonstopmode"));
        if let Some(ref p) = self.build_directory {
            let mut flag = OsString::from("-output-directory=");
            flag.push(p.as_os_str());
            cmd.arg(&flag);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());
        cmd
    }
}
