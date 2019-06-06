

use std::ffi::OsString;


use failure::{Error, err_msg};

pub enum LaTeXEngine {
    Pdflatex,
    Luatex,
    Pdftex,
}



pub fn get_extension_for_engine(engine: &str) -> Result<OsString, Error> {
    match engine {
        "pdflatex" => Ok(OsString::from(".pdf")),
        "pdftex"   => Ok(OsString::from(".pdf")),
        "luatex"   => Ok(OsString::from(".pdf")),
        _          => Err(err_msg(format!("Unrecognised LaTeX engine: {}", engine)))
    }
}