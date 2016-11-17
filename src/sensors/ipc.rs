use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};
use message::{Message, WebpackInfo};
use serde_json as json;
use std::error::Error;
use std::io::{Read, Write, self};
use std::str::FromStr;
use std::sync::mpsc;
use std::{result, thread};
use unix_socket::{UnixStream, UnixListener};
use error::BarError;
use std::fs;

pub struct Ipc {}

const UNIX_SOCK: &'static str = "/tmp/rubar.sock";

type RunResult = result::Result<(), Box<Error>>;

impl Ipc {
    pub fn new() -> Ipc {
        Ipc {}
    }

    pub fn run(&self, tx: mpsc::Sender<Message>) -> RunResult {

        fs::remove_file(UNIX_SOCK)?;
        let listener = UnixListener::bind(UNIX_SOCK)?;

        thread::spawn(move || {

            for stream in listener.incoming() {
                let tx = tx.clone();
                match stream {
                    Ok(stream) => {
                        thread::spawn(move || {
                            handle_client(stream, tx);
                        });
                    }
                    Err(err) => {
                        println!("Ipc ERROR: {}", err);
                    }
                }
            }

        });

        Ok(())
    }
}

fn handle_client(mut stream: UnixStream, tx: mpsc::Sender<Message>) {

    fn build_event(msgtype: u32, payload: &str) -> Result<Message, Box<Error>> {
        Ok(match msgtype {
            c @ 0 ... 20 => return Err(From::from(
                format!("reserved code range {}", c))),
            21 => Message::Webpack(WebpackInfo::from_str(payload)?),
            _ => unreachable!(),
        })
    }

    loop {
        match stream.receive_i3_message() {
            Ok((msgint, payload)) => {
                // strip the highest order bit indicating it's an event.
                let msgtype = (msgint << 1) >> 1;

                match build_event(msgtype, &payload) {
                    Ok(event) => tx.send(event).unwrap(),
                    Err(e) => println!("ipc ERROR: {}", e),
                }
            },

            Err(e) => println!("ipc ERROR: {}", e),
        }
    }
}

/// Data for `WebpackEvent`.
impl FromStr for WebpackInfo {
    type Err = BarError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data: json::Value = json::from_str(s)?;
        let obj = data.as_object().ok_or(
            BarError::Bar(format!("webpack json not object")))?;
        let val = obj.get("change").unwrap();
        let change = match *val {
            json::Value::String(ref v) => v,
            _ => Err(BarError::Bar(format!("change field missing String")))?,
        };
        Ok(match change.as_ref() {
            "compile" => WebpackInfo::Compile,
            "done" => WebpackInfo::Done,
            _ => unreachable!()
        })
    }
}


trait I3Funcs {
    fn send_i3_message(&mut self, u32, &str) -> io::Result<()>;
    fn receive_i3_message(&mut self) -> io::Result<(u32, String)>;
}

impl I3Funcs for UnixStream {
    fn send_i3_message(&mut self, message_type: u32, payload: &str) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(14 + payload.len());
        bytes.extend("i3-ipc".bytes());                              // 6 bytes
        bytes.write_u32::<LittleEndian>(payload.len() as u32)?; // 4 bytes
        bytes.write_u32::<LittleEndian>(message_type)?;         // 4 bytes
        bytes.extend(payload.bytes());                               // payload.len() bytes
        self.write_all(&bytes[..])
    }

    /// returns a tuple of (message type, payload)
    fn receive_i3_message(&mut self) -> io::Result<(u32, String)> {
        let mut magic_data = [0_u8; 6];
        self.read_exact(&mut magic_data)?;
        let magic_string = String::from_utf8_lossy(&magic_data);
        if magic_string != "i3-ipc" {
            let error_text = format!("unexpected magic string: expected 'i3-ipc' but got {}",
                                     magic_string);
            return Err(io::Error::new(io::ErrorKind::Other, error_text));
        }
        let payload_len = self.read_u32::<LittleEndian>()?;
        let message_type = self.read_u32::<LittleEndian>()?;
        let mut payload_data = vec![0_u8 ; payload_len as usize];
        self.read_exact(&mut payload_data[..])?;
        let payload_string = String::from_utf8_lossy(&payload_data).into_owned();
        Ok((message_type, payload_string))
    }
}
