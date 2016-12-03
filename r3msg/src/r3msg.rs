extern crate unix_socket;
extern crate i3ipc;
extern crate serde_json;
extern crate r3bar;

use i3ipc::{reply};
use r3bar::r3ipc::R3Funcs;
use serde_json as json;
use std::error::Error;
use std::fmt;
use std::io;
use unix_socket::UnixStream;

pub const R3_UNIX_SOCK: &'static str = "/tmp/rubar.sock";

#[derive(Debug)]
pub enum EstablishError {
    /// An error while getting the socket path
    GetSocketPathError(io::Error),
    /// An error while accessing the socket
    SocketError(io::Error)
}

pub struct R3Msg {
    stream: UnixStream
}

impl R3Msg {

    pub fn new() -> Result<Self, EstablishError> {
        match UnixStream::connect(R3_UNIX_SOCK) {
            Ok(stream) => Ok(R3Msg{stream: stream}),
            Err(error) => Err(EstablishError::SocketError(error))
        }
    }


    pub fn send_msg(&mut self, msgtype: u32, payload: &str) -> Result<reply::Command, MessageError> {

        if let Err(e) = self.stream.send_i3_message(msgtype, payload) {
            return Err(MessageError::Send(e));
        }

        // could check that msgtype is REPLY
        let payload = match self.stream.receive_i3_message() {
            Ok((_, payload)) => payload,
            Err(e) => { return Err(MessageError::Receive(e)); }
        };

        let j = match json::from_str::<json::Value>(&payload) {
            Ok(v) => v,
            Err(e) => { return Err(MessageError::JsonCouldntParse(e)); }
        };

        // assumes valid json contents
        let commands = j.as_array().unwrap();
        let vec: Vec<_>
            = commands.iter()
            .map(|c| c.as_object().unwrap())
            .map(|c|
                 reply::CommandOutcome {
                     success: c.get("success").unwrap().as_bool().unwrap(),
                     error: match c.get("error") {
                         Some(val) => Some(val.as_str().unwrap().to_owned()),
                         None => None
                     }
                 })
            .collect();

        Ok(reply::Command { outcomes: vec })
    }
}

/*
 * Message error can be recycled from i3ipc-rs once it updates serde_json.
 *
 */
// An error sending or receiving a message.
#[derive(Debug)]
pub enum MessageError {
    /// Network error sending the message.
    Send(io::Error),
    /// Network error receiving the response.
    Receive(io::Error),
    /// Got the response but couldn't parse the JSON.
    JsonCouldntParse(json::Error),
}

impl Error for MessageError {
    fn description(&self) -> &str {
        match *self {
            MessageError::Send(_) => "Network error while sending message to i3",
            MessageError::Receive(_) => "Network error while receiving message from i3",
            MessageError::JsonCouldntParse(_) => "Got a response from i3 but couldn't parse the JSON",
        }
    }
    fn cause(&self) -> Option<&Error> {
        match *self {
            MessageError::Send(ref e) => Some(e),
            MessageError::Receive(ref e) => Some(e),
            MessageError::JsonCouldntParse(ref e) => Some(e),
        }
    }
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
