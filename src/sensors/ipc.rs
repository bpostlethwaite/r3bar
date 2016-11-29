use message::{Message, WebpackInfo};
use std::error::Error;
use std::fs;
use std::str::FromStr;
use std::sync::mpsc;
use std::{result, thread};
use unix_socket::{UnixStream, UnixListener};
use r3ipc::{R3Funcs, R3_UNIX_SOCK};


pub struct Ipc {}

type RunResult = result::Result<(), Box<Error>>;

impl Ipc {
    pub fn new() -> Ipc {
        Ipc {}
    }

    pub fn run(&self, tx: mpsc::Sender<Message>) -> RunResult {

        fs::remove_file(R3_UNIX_SOCK).ok();
        let listener = UnixListener::bind(R3_UNIX_SOCK)?;

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

fn build_event(msgtype: u32, payload: &str) -> Result<Message, Box<Error>> {
    Ok(match msgtype {
        c @ 0 ... 20 => return Err(From::from(
            format!("reserved code range {}", c))),
        21 => Message::Webpack(WebpackInfo::from_str(payload)?),
        22 => Message::Unpark,
        _ => unreachable!(),
    })
}

fn handle_client(mut stream: UnixStream, tx: mpsc::Sender<Message>) {

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

            Err(e) => {
                // stop listening to this connection on error. Client will have
                // to reconnect.
                println!("ipc ERROR: {}", e);
                break;
            },
        }
    }
}
