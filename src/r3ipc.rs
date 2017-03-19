/*
 * Some code in this file has been taken and modifed from
 * https://github.com/tmerr/i3ipc-rs
 */

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use i3ipc::{reply};
use serde_json as json;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write, self};
use unix_socket::UnixStream;

pub const R3_UNIX_SOCK: &'static str = "/tmp/rubar.sock";

pub const RESERVED: u32 = 20;
pub const REPLY: u32 = 21;
pub const UNPARK: u32 = 22;
pub const WEBPACK: u32 = 23;
pub const TICKER: u32 = 24;

pub trait R3Funcs {
    fn send_i3_message(&mut self, u32, &str) -> io::Result<()>;
    fn receive_i3_message(&mut self) -> io::Result<(u32, String)>;
}

impl R3Funcs for UnixStream {

    fn send_i3_message(&mut self, message_type: u32, payload: &str) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(14 + payload.len());
        bytes.extend("i3-ipc".bytes());                         // 6 bytes
        bytes.write_u32::<LittleEndian>(payload.len() as u32)?; // 4 bytes
        bytes.write_u32::<LittleEndian>(message_type)?;         // 4 bytes
        bytes.extend(payload.bytes());                          // payload.len() bytes
        self.write_all(&bytes[..])
    }

    /// returns a tuple of (message type, payload)
    fn receive_i3_message(&mut self) -> io::Result<(u32, String)> {
        let mut magic_data = [0_u8; 6];
        if let Err(e) = self.read_exact(&mut magic_data) {
            return Err(e);
        }
        let magic_string = String::from_utf8_lossy(&magic_data);
        if magic_string != "i3-ipc" {
            let error_text = format!(
                "unexpected magic string: expected 'i3-ipc' but got {}",
                magic_string
            );
            return Err(io::Error::new(io::ErrorKind::Other, error_text));
        }
        let payload_len = self.read_u32::<LittleEndian>()?;
        let message_type = self.read_u32::<LittleEndian>()?;
        let mut payload_data = vec![0_u8 ; payload_len as usize];
        if let Err(e) = self.read_exact(&mut payload_data[..]) {
            return Err(e);
        };
        let payload_string = String::from_utf8_lossy(&payload_data).into_owned();
        Ok((message_type, payload_string))
    }
}

pub struct R3Msg {
    stream: UnixStream
}

impl R3Msg {

    pub fn new(socket_path: Option<&str>) -> Result<Self, io::Error> {

        let socket_path = socket_path.unwrap_or(R3_UNIX_SOCK);
        UnixStream::connect(socket_path).map(|stream| R3Msg{stream: stream})
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
