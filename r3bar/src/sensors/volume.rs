use std::error::Error;
use std::process::Command;
use std::str::from_utf8;
use std::sync::mpsc::{Sender};
use std::time::Duration;
use std::thread;
use error::BarError;

#[derive(Debug)]
pub struct Volume {
    interval: Duration,
}

impl Volume {

    pub fn new(interval: Duration) -> Self {
        Volume{interval: interval}
    }

    pub fn run<T, F>(&self, tx: Sender<T>, f: F) -> Result<thread::JoinHandle<T>, Box<Error>>
        where F: 'static + Send + Fn(String) -> T,
              T: 'static + Send
    {

        let iv = self.interval;
        tx.send(f(get_volume()?))?;

        Ok(thread::spawn(move || {

            loop {
                if let Err(e) = get_volume().map_err(|e| e.to_string())
                    .and_then(|vol| tx.send(f(vol)).map_err(|e| e.to_string())) {
                        println!("volume sensor ERROR: {}", e);
                    }

                thread::park_timeout(iv);
            }
        }))
    }
}


fn get_volume() -> Result<String, BarError> {
    let output = Command::new("vol")
        .arg("get")
        .output()?;

    if !output.status.success() {
        return Err(
            BarError::Bar(
                format!("'vol get' ERROR: {}", from_utf8(&output.stderr)?)));
    }

    from_utf8(&output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| BarError::Utf8(e))
}
