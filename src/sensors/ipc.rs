use error::BarError;
use message::{Message, WebpackInfo};
use r3ipc::{R3Funcs, R3_UNIX_SOCK, self};
use sensors::{Sensor, SensorResult};
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::{thread};
use unix_socket::{UnixStream, UnixListener};


pub struct Ipc {
    pub socket_path: String,
}

pub struct R3Msg {
    pub msgtype: u32,
    pub payload: String,
}

impl Ipc {
    pub fn new() -> Result<Self, BarError> {
        Ok(Ipc {
            socket_path: R3_UNIX_SOCK.to_owned(),
        })
    }
}

impl Sensor for Ipc {
    fn run(&self, tx: mpsc::Sender<Message>) -> SensorResult {
        let socket_path = Path::new(&self.socket_path);
        fs::remove_file(socket_path).ok();
        let listener = UnixListener::bind(socket_path)?;

        Ok(thread::spawn(move|| {

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

            Ok(())
        }))
    }
}


fn handle_client(mut stream: UnixStream, tx: mpsc::Sender<Message>) {

    loop {
        match stream.receive_i3_message() {
            Ok((msgint, payload)) => {
                // strip the highest order bit indicating it's an event.
                let msgtype = (msgint << 1) >> 1;

                let message = to_message(msgtype, payload);
                let message_err = message.is_err();
                let reply = match message {
                    Ok(msg) => {
                        tx.send(msg).unwrap();
                        stream.send_i3_message(r3ipc::REPLY, &reply_ok())
                    },
                    Err(e) => {
                        tx.send(Message::Error(e.to_string())).unwrap();
                        stream.send_i3_message(
                            r3ipc::REPLY, &reply_err(e.to_string())
                        )
                    },
                };

                if let Err(e) = reply {
                    tx.send(Message::Error(e.to_string())).unwrap();
                    break;
                };

                // close the stream if there was a message in the error;
                if message_err {
                    break;
                }
            },

            Err(e) => {
                // stop listening to this connection on error
                tx.send(Message::Error(e.to_string())).unwrap();
                break;
             },
        }
    }
}

fn to_message(msgtype: u32, payload: String) -> Result<Message, BarError> {
    match msgtype {
        c @ 0...r3ipc::RESERVED => Err(BarError::Bar(
            format!("R3Msg: reserved code range {}", c))),
        r3ipc::WEBPACK => Ok(Message::Webpack(WebpackInfo::from_str(&payload)?)),
        r3ipc::UNPARK => Ok(Message::Unpark),
        _ => Err(BarError::Bar(
            format!("R3Msg: msgtype '{}' not implemented", msgtype))),
    }
}



// see https://i3wm.org/docs/ipc.html#_command_reply
fn reply_ok<'a>() -> String {
    return "[{ \"success\": true }]".to_string();
}

// see https://i3wm.org/docs/ipc.html#_command_reply
fn reply_err<'a>(errmsg: String) -> String {
    return format!("[{{ \"success\": false, \"error\": \"{}\" }}]", errmsg);
}
