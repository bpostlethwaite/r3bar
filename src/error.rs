use conrod;
use std::error;
use std::fmt;
use std::io;
use std::str;
use serde_json;
use i3ipc;


#[derive(Debug)]
pub enum BarError {
    Io(io::Error),
    Font(conrod::text::font::Error),
    Utf8(str::Utf8Error),
    Json(serde_json::error::Error),
    I3Establish(i3ipc::EstablishError),
    I3Message(i3ipc::MessageError),
    Bar(String),
}

impl fmt::Display for BarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BarError::Io(ref err) => write!(f, "IO error: {}", err),
            BarError::Json(ref err) => write!(f, "Json error: {}", err),
            BarError::I3Establish(ref err) => write!(
                f, "I3 establish error: {}", err),
            BarError::I3Message(ref err) => write!(
                f, "I3 message error: {}", err),
            BarError::Font(ref err) => write!(f, "Font error: {}", err),
            BarError::Utf8(ref err) => write!(f, "Utf8 error: {}", err),
            BarError::Bar(ref err) => write!(f, "Bar error: {}", err),
        }
    }
}

impl error::Error for BarError {
    fn description(&self) -> &str {
        match *self {
            BarError::Io(ref err) => err.description(),
            BarError::Json(ref err) => err.description(),
            BarError::I3Establish(ref err) => err.description(),
            BarError::I3Message(ref err) => err.description(),
            BarError::Font(ref err) => err.description(),
            BarError::Utf8(ref err) => err.description(),
            BarError::Bar(ref err) => err,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            BarError::Io(ref err) => Some(err),
            BarError::Json(ref err) => Some(err),
            BarError::I3Establish(ref err) => Some(err),
            BarError::I3Message(ref err) => Some(err),
            BarError::Font(ref err) => Some(err),
            BarError::Utf8(ref err) => Some(err),
            BarError::Bar(_) => Some(self),
        }
    }
}

impl From<io::Error> for BarError {
    fn from(err: io::Error) -> BarError {
        BarError::Io(err)
    }
}

impl From<serde_json::error::Error> for BarError {
    fn from(err: serde_json::error::Error) -> BarError {
        BarError::Json(err)
    }
}

impl From<conrod::text::font::Error> for BarError {
    fn from(err: conrod::text::font::Error) -> BarError {
        BarError::Font(err)
    }
}

impl From<i3ipc::EstablishError> for BarError {
    fn from(err: i3ipc::EstablishError) -> BarError {
        BarError::I3Establish(err)
    }
}


impl From<i3ipc::MessageError> for BarError {
    fn from(err: i3ipc::MessageError) -> BarError {
        BarError::I3Message(err)
    }
}

impl From<str::Utf8Error> for BarError {
    fn from(err: str::Utf8Error) -> BarError {
        BarError::Utf8(err)
    }
}

impl From<String> for BarError {
    fn from(err: String) -> BarError {
        BarError::Bar(err)
    }
}
