use std::error::Error;
use std::fmt::{self, Display};
use std::fs;
use std::path::Path;

use crate::problem::Problem;
use crate::problem_parser::{ParseError, parse_problem};

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Parse(ParseError),
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "erro de arquivo: {error}"),
            Self::Parse(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ParseError> for AppError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

pub fn read_problem(path: impl AsRef<Path>) -> Result<Problem, AppError> {
    Ok(parse_problem(&fs::read_to_string(path)?)?)
}

pub fn write_problem(path: impl AsRef<Path>, problem: &Problem) -> Result<(), AppError> {
    fs::write(path, problem.to_string())?;
    Ok(())
}
