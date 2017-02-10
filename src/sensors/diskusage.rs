use error::BarError;
use sensors;
use message::Message;
use std::process::Command;
use std::str::from_utf8;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub struct DiskUsage {
    interval: Duration,
    mountpoints: Vec<String>,
}

impl DiskUsage {
    pub fn new(interval: Duration, mountpoints: Vec<String>) -> Self {
        DiskUsage{interval: interval, mountpoints: mountpoints}
    }
}

impl sensors::Sensor for DiskUsage {
    fn run(&self, tx: mpsc::Sender<Message>) -> sensors::SensorResult {

        let iv = self.interval;
        let mountpoints = self.mountpoints.clone();

        tx.send(Message::DiskUsage(get_usage(mountpoints.clone())?)).unwrap();

        Ok(thread::spawn(move || {
            loop {
                if let Err(e) = get_usage(mountpoints.clone()).map_err(|e| e.to_string())
                    .and_then(|usage| tx.send(Message::DiskUsage(usage))
                              .map_err(|e| e.to_string())) {
                        println!("volume sensor ERROR: {}", e);
                    }

                thread::park_timeout(iv);
            }
        }))
    }
}

const DF_USAGE_COL: usize = 4;
const DF_MOUNT_COL: usize = 5;

fn get_usage(mountpoints: Vec<String>) -> Result<String, BarError> {
    let output = Command::new("df")
        .arg("-h")
        .output()?;

    if !output.status.success() {
        return Err(
            BarError::Bar(
                format!(
                    "'DiskUsage df' ERROR: {}", from_utf8(&output.stderr)?)));
    }

    let extract_mounts = |s: &str| -> String {

        let mut usages = Vec::new();
        let lines: Vec<&str> = s.split("\n").collect();

        for mp in mountpoints {
            for line in &lines {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() > DF_MOUNT_COL &&
                    fields[DF_MOUNT_COL] == mp {
                        usages.push(format!("{} {}", mp, fields[DF_USAGE_COL]));
                    }
            }
        }

        return usages.join("  ");
    };


    from_utf8(&output.stdout)
        .map(extract_mounts)
        .map(|s| s.trim().to_owned())
        .map_err(|e| BarError::Utf8(e))
}
