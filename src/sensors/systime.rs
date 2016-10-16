use chrono::Local;
use std::thread;
use std::time::Duration;
use std::sync::mpsc;

use message::Message;

pub struct SysTime {}

impl SysTime {
    pub fn run(&self, tx: mpsc::Sender<Message>) {
        thread::spawn(move || {

            let iv = Duration::from_millis(100);

            loop {
                let dt = Local::now();
                let time_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                tx.send(Message::Time(time_str)).unwrap();

                thread::sleep(iv);
            }
        });
    }
}
