use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, self};
use std::sync::mpsc::{Sender};
use std::thread;
use std::time::Duration;

pub struct Battery {
    pub interval: Duration
}

static CAPACITY_PATH: &'static str = "/sys/class/power_supply/BAT1/capacity";
static AC_PATH: &'static str = "/sys/class/power_supply/AC/online";
static STATUS_PATH: &'static str = "/sys/class/power_supply/BAT1/status";

impl Battery {

    pub fn new(interval: Duration) -> Battery {
        Battery{interval: interval}
    }

    pub fn run<T, F>(&self, tx: Sender<T>, f: F) -> Result<(), Box<Error>>
        where F: 'static + Send + Fn((String, String, String)) -> T,
              T: 'static + Send,
    {
        let iv = self.interval;

        // if we can't read these paths return early
        let _ = read_info_file(CAPACITY_PATH)?;
        let _ = read_info_file(STATUS_PATH)?;
        let _ = read_info_file(AC_PATH)?;

        thread::spawn(move || {
            loop {
                if let Err(e) = || -> Result<(), Box<Error>> {
                    let capacity = read_info_file(CAPACITY_PATH)?;
                    let status = read_info_file(STATUS_PATH)?;
                    let ac = read_info_file(AC_PATH)?;

                    tx.send(f((capacity, status, ac)))?;
                    Ok(())
                }() {
                    println!("Battery Sensor Error: {}", e);
                }

                thread::sleep(iv);
            }
        });

        Ok(())
    }
}


fn read_info_file(file_path: &'static str) -> Result<String, io::Error> {
    let f = File::open(file_path)?;
    let mut input = String::new();

    // read the input.
    let mut reader = BufReader::new(f);
    reader.read_line(&mut input)?;

    // clean newlines and whitespace
    Ok(input.trim().to_owned())
}
