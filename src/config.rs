use std::process::Command;
use std::slice::Iter;
use std::path::PathBuf;

use crate::cli::CliOptions;


pub struct Config {

    cli_options: CliOptions,


}

impl Default for Config {
    fn default() -> Config {
        Config {
            cli_options: CliOptions::default(),
        }
    }
}


impl Config {

    pub fn new(cli_options: CliOptions) -> Config {
        Config {
            cli_options,
        }
    }

    pub fn get_command(&self) -> Command {
        self.cli_options.get_command()
    }

    pub fn paths(&self) -> Iter<PathBuf> {
        self.cli_options.paths()
    }

}