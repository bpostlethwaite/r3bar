use conrod;
use std::error::{self, Error};
use std::fmt;
use std::io;
use std::str;
use serde_json::error::{Error as JsonError};
use std::sync::mpsc::SendError;

#[derive(Debug)]
pub enum BarError {
    Io(io::Error),
    Font(conrod::text::font::Error),
    Utf8(str::Utf8Error),
    Json(JsonError),
    Bar(String),
    Box(Box<Error>),
}

impl fmt::Display for BarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BarError::Io(ref err) => write!(f, "IO error: {}", err),
            BarError::Json(ref err) => write!(f, "Json error: {}", err),
            BarError::Font(ref err) => write!(f, "Font error: {}", err),
            BarError::Utf8(ref err) => write!(f, "Utf8 error: {}", err),
            BarError::Box(ref err) => write!(f, "Boxed error: {}", err),
            BarError::Bar(ref err) => write!(f, "Bar error: {}", err),
        }
    }
}

impl error::Error for BarError {
    fn description(&self) -> &str {
        match *self {
            BarError::Io(ref err) => err.description(),
            BarError::Json(ref err) => err.description(),
            BarError::Font(ref err) => err.description(),
            BarError::Utf8(ref err) => err.description(),
            BarError::Box(ref err) => err.description(),
            BarError::Bar(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            BarError::Io(ref err) => Some(err),
            BarError::Json(ref err) => Some(err),
            BarError::Font(ref err) => Some(err),
            BarError::Utf8(ref err) => Some(err),
            BarError::Box(ref err) => Some(err.as_ref()),
            BarError::Bar(_) => Some(self),
        }
    }
}

impl From<io::Error> for BarError {
    fn from(err: io::Error) -> BarError {
        BarError::Io(err)
    }
}

impl From<JsonError> for BarError {
    fn from(err: JsonError) -> BarError {
        BarError::Json(err)
    }
}


impl From<conrod::text::font::Error> for BarError {
    fn from(err: conrod::text::font::Error) -> BarError {
        BarError::Font(err)
    }
}

impl From<str::Utf8Error> for BarError {
    fn from(err: str::Utf8Error) -> BarError {
        BarError::Utf8(err)
    }
}

impl From<Box<Error>> for BarError {
    fn from(err: Box<Error>) -> BarError {
        BarError::Box(err)
    }
}

impl From<String> for BarError {
    fn from(err: String) -> BarError {
        BarError::Bar(err)
    }
}
