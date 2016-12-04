use std::error::Error;
use std::fs;
use std::sync::mpsc;
use std::{result, thread};
use unix_socket::{UnixStream, UnixListener};
use r3ipc::{R3Funcs, R3_UNIX_SOCK, self};


pub struct Ipc {
    pub socket: UnixListener,
}

pub struct R3Msg {
    pub msgtype: u32,
    pub payload: String,
}

type RunResult = result::Result<(), Box<Error>>;

impl Ipc {
    pub fn new() -> Result<Self, Box<Error>> {
        fs::remove_file(R3_UNIX_SOCK).ok();
        let listener = UnixListener::bind(R3_UNIX_SOCK)?;

        Ok(Ipc {
            socket: listener,
        })
    }

    pub fn run<T, F>(self, tx: mpsc::Sender<T>, f: F) -> RunResult
        where F: 'static + Send + Copy + Fn(R3Msg) -> T,
              T: 'static + Send,
    {
        thread::spawn(move || {

            for stream in self.socket.incoming() {
                let tx = tx.clone();
                match stream {
                    Ok(stream) => {
                        thread::spawn(move || {
                            handle_client(stream, tx, f);
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


fn handle_client<T, F>(mut stream: UnixStream, tx: mpsc::Sender<T>, f: F)
        where F: 'static + Send + Fn(R3Msg) -> T,
              T: 'static + Send,
{

    loop {
        match stream.receive_i3_message() {
            Ok((msgint, payload)) => {
                // strip the highest order bit indicating it's an event.
                let msgtype = (msgint << 1) >> 1;

                let result = tx.send(f(R3Msg{
                    msgtype: msgtype,
                    payload: payload,
                }));

                if let Err(e) = match result {
                    Ok(_) => stream
                        .send_i3_message(r3ipc::REPLY, &reply_ok()),

                    Err(e) => stream
                        .send_i3_message(r3ipc::REPLY, &reply_err(e.to_string())),
                } {
                    // stop listening to this connection on error and force client
                    // reconnect
                    println!("ipc REPLY ERROR: {}", e);
                    break;
                }
            },

            Err(e) => {
                // stop listening to this connection on error and force client
                // reconnect
                println!("ipc COMMAND ERROR: {}", e);
                break;
             },
        }
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
