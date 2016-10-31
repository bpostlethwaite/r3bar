use chrono::Local;
use std::error::Error;
use std::marker::Send;
use std::sync::mpsc;
use std::time::Duration;
use std::{result, thread};

pub struct SysTime {}

type RunResult = result::Result<(), Box<Error>>;

impl SysTime {
    pub fn run<F, T: 'static + Send>(&self, tx: mpsc::Sender<T>, f: F) -> RunResult
        where F: 'static + Send + Fn(String) -> T
    {
        thread::spawn(move || {

            let iv = Duration::from_millis(100);

            loop {
                let dt = Local::now();
                let time_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();

                if let Err(_) = tx.send(f(time_str)) {
                    continue; // Logging?
                }

                thread::sleep(iv);
            }
        });

        Ok(())
    }
}
