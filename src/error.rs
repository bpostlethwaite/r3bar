use conrod;
use std::error;
use std::fmt;
use std::io;
use serde_json::error::{Error as JsonError};

#[derive(Debug)]
pub enum BarError {
    Io(io::Error),
    Font(conrod::text::font::Error),
    Json(JsonError),
    Bar(String),
}

impl fmt::Display for BarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            // Both underlying errors already impl `Display`, so we defer to
            // their implementations.
            BarError::Io(ref err) => write!(f, "IO error: {}", err),
            BarError::Json(ref err) => write!(f, "Json error: {}", err),
            BarError::Font(ref err) => write!(f, "Font error: {}", err),
            BarError::Bar(ref err) => write!(f, "Bar error: {}", err),
        }
    }
}

impl error::Error for BarError {
    fn description(&self) -> &str {
        // Both underlying errors already impl `Error`, so we defer to their
        // implementations.
        match *self {
            BarError::Io(ref err) => err.description(),
            BarError::Json(ref err) => err.description(),
            BarError::Font(ref err) => err.description(),
            BarError::Bar(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            // N.B. Both of these implicitly cast `err` from their concrete
            // types (either `&io::Error` or `&num::FontIntError`)
            // to a trait object `&Error`. This works because both error types
            // implement `Error`.
            BarError::Io(ref err) => Some(err),
            BarError::Json(ref err) => Some(err),
            BarError::Font(ref err) => Some(err),
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
