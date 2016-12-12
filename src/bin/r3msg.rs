extern crate r3bar;
extern crate i3ipc;

use r3bar::r3ipc::{R3Msg};
use std::env;

fn help() {
    println!("usage:
msgtype <integer>
    msgtype number - see r3ipc documentation.
payload [string]
    msg arguments if any.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let empty_payload = "".to_string();

    match args.len() {
        // no arguments passed
        1 => {
            help();
        },

        l @ 2...3 => {

            let cmd = &args[1];
            let payload;
            if l == 3 {
                payload = &args[2];
            } else {
                payload = &empty_payload;
            }

            match cmd.parse::<u32>() {
                Ok(msgtype) => send(msgtype, payload),
                Err(_) => return help(),
            }
        },

        // all the other cases
        _ => {
            // show a help message
            help();
        }
    }
}

fn send(msgtype: u32, payload: &str) {
    match R3Msg::new(None).unwrap().send_msg(msgtype, payload) {
        Ok(i3ipc::reply::Command{outcomes}) => println!("{:?}", outcomes),
        Err(e) => println!("{}", e)
    }
}
