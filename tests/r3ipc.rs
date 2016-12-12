extern crate r3bar;
extern crate rand;
extern crate i3ipc;

use rand::{thread_rng, Rng};

use r3bar::r3ipc;
use r3bar::message::Message;
use r3bar::sensors::{Sensor, ipc};
use std::sync::mpsc;
use std::{thread};

#[test]
fn r3msg() {
    let socket_path = &tmp_socket_path();
    let ipc = ipc::Ipc::new(Some(socket_path)).unwrap();
    let (tx, rx) = mpsc::channel();
    ipc.run(tx);

    let nmsgs = 1;
    let main_handle = thread::spawn(move || {

        let mut messages = Vec::new();

        for i in 0..nmsgs {
            let msg = match rx.recv() {
                Err(e) => panic!(e),
                Ok(msg) => msg,
            };

            messages.push(msg);
        }

        return messages;
    });

    let mut r3msg = r3ipc::R3Msg::new(Some(socket_path)).unwrap();

    let reply = r3msg.send_msg(r3ipc::UNPARK, "").unwrap();
    assert_reply_success(reply);

    let messages = main_handle.join().unwrap();
    let msg = &messages[0];
    match *msg {
        Message::Unpark => (),
        _ => panic!("expected Mesage::Unpark"),
    };
}

fn assert_reply_success(reply: i3ipc::reply::Command) {
    let outcome = reply.outcomes.iter().next().unwrap();
    assert!(outcome.success);
}

fn tmp_socket_path() -> String {
    let rand_str: String = thread_rng().gen_ascii_chars().take(10).collect();
    format!("/tmp/{}", rand_str).to_owned()
}
