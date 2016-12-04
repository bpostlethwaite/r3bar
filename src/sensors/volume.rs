use error::BarError;
use sensors;
use message::Message;
use std::process::Command;
use std::str::from_utf8;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub struct Volume {
    interval: Duration,
}

impl Volume {

    pub fn new(interval: Duration) -> Self {
        Volume{interval: interval}
    }
}

impl sensors::Sensor for Volume {
    fn run(&self, tx: mpsc::Sender<Message>) -> sensors::SensorResult {

        let iv = self.interval;
        tx.send(Message::Volume(get_volume()?)).unwrap();

        Ok(thread::spawn(move || {
            loop {
                if let Err(e) = get_volume().map_err(|e| e.to_string())
                    .and_then(|vol| tx.send(Message::Volume(vol)).map_err(|e| e.to_string())) {
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
