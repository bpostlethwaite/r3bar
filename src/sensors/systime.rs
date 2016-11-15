use chrono::Local;
use std::error::Error;
use std::marker::Send;
use std::sync::mpsc;
use std::time::Duration;
use std::{result, thread};

pub struct SysTime {
    pub interval: Duration
}

type RunResult = result::Result<(), Box<Error>>;

impl SysTime {
    pub fn run<F, T: 'static + Send>(&self, tx: mpsc::Sender<T>, f: F) -> RunResult
        where F: 'static + Send + Fn(String) -> T
    {
        let iv = self.interval;

        thread::spawn(move || {
            loop {
                let dt = Local::now();
                let time_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();

                if let Err(_) = tx.send(f(time_str)) {
                    continue; // TODO Logging?
                }

                thread::sleep(iv);
            }
        });

        Ok(())
    }
}
