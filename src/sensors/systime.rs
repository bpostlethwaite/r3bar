use chrono::Local;
use message::Message;
use sensors::{Sensor, SensorResult};
use std::sync::mpsc;
use std::time::Duration;
use std::{thread};

pub struct SysTime {
    pub interval: Duration
}

impl SysTime {
    pub fn new(interval: Duration) -> SysTime {
        SysTime{interval: interval}
    }
}

impl Sensor for SysTime {

    fn run(&self, tx: mpsc::Sender<Message>) -> SensorResult {
        let iv = self.interval;

        Ok(thread::spawn(move || {
            loop {
                let dt = Local::now();
                let time_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();

                if let Err(e) = tx.send(Message::Time(time_str)) {
                    println!("SysTime ERROR: {}", e); // TODO Logging?
                }

                thread::park_timeout(iv);
            }
        }))
    }
}
