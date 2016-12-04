// adapted from https://github.com/ultrabug/py3status
// @original author Markus Weimar <mail@markusweimar.de>
// @license BSD
//
use message::Message;
use sensors::{Sensor, SensorResult};
use regex::Regex;
use std::error::Error;
use std::process::Command;
use std::str::from_utf8;
use std::sync::mpsc::{Sender};
use std::time::Duration;
use std::thread;

#[derive(Debug)]
pub struct ConfigureWifi {
    bitrate_degraded: f64,
    interval: Duration,
    ip: bool,
    device: String,
}

impl ConfigureWifi {
    pub fn new() -> Result<ConfigureWifi, Box<Error>> {

        // Try to guess the interface. If `iw dev` fails we fail the whole
        // build as this module is based on this command.
        //

        let output = Command::new("iw").arg("dev")
            .output()?;

        if !output.status.success() {
            return Err(From::from(format!("iw dev error: {}", from_utf8(&output.stderr)?)));
        }

        let re = Regex::new(r"Interface\s*([^\s]+)")?;

        // attempt to find the correct interface
        let device = re.captures(from_utf8(&output.stdout)?)
            .and_then(|captures| captures.at(1))
            .map_or("wlan0", |c| c);

        Ok(ConfigureWifi {
            bitrate_degraded: 53.,
            interval: Duration::from_millis(5000),
            ip: false,
            device: device.to_string(),
        })
    }

    pub fn bitrate_degraded(&mut self, bitrate: f64) -> &mut ConfigureWifi {
        self.bitrate_degraded = bitrate;
        self
    }

    pub fn interval(&mut self, interval: Duration) -> &mut ConfigureWifi {
        self.interval = interval;
        self
    }

    pub fn ip(&mut self, ip: bool) -> &mut ConfigureWifi {
        self.ip = ip;
        self
    }

    pub fn device<'a>(&'a mut self, device: String) -> &'a mut ConfigureWifi {
        self.device = device;
        self
    }

    pub fn configure(&self) -> Wifi {
        return Wifi {
            bitrate_degraded: self.bitrate_degraded,
            interval: self.interval,
            ip: self.ip,
            device: self.device.clone(),
            ssid: None,
            max_bitrate: self.bitrate_degraded,
        };
    }
}

#[derive(Clone, Debug)]
pub struct WifiStatus {
    pub ssid: Option<String>,
    pub ip: Option<String>,
    pub signal: Option<f64>,
    pub quality: i64,
    max_bitrate: f64,
}

impl WifiStatus {
    pub fn new(bitrate: f64) -> WifiStatus {
        WifiStatus {
            max_bitrate: bitrate,
            ssid: None,
            ip: None,
            signal: None,
            quality: 0,
        }
    }
}

#[derive(Debug)]
pub struct Wifi {
    bitrate_degraded: f64,
    interval: Duration,
    ip: bool,
    device: String,
    ssid: Option<String>,
    max_bitrate: f64,
}

impl Sensor for Wifi {
    fn run(&self, tx: Sender<Message>) -> SensorResult {

        let iv = self.interval;
        let dev = self.device.clone();
        let ip = self.ip;
        let degraded = self.bitrate_degraded;

        Ok(thread::spawn(move || {

            // send a snapshot of current workspace immediately
            let mut last_status = WifiStatus::new(degraded);

            loop {

                if let Err(e) = get_wifi_status(
                    dev.clone(), ip, degraded, last_status.clone())
                    .and_then(|status| {
                        last_status = status.clone();
                        tx.send(Message::Wifi(status)).map_err(|e| From::from(e))
                    }) {
                        println!("wifistatus ERROR: {}", e); // TODO LOGGING
                    }

                thread::sleep(iv);
            }
        }))
    }
}


fn get_wifi_status(device: String,
                   do_ip: bool,
                   bitrate_degraded: f64,
                   last_status: WifiStatus)
                   -> Result<WifiStatus, Box<Error>> {

    let output = Command::new("iw").arg("dev")
        .arg(&device)
        .arg("link")
        .output()?;

    if !output.status.success() {
        return Err(From::from(format!("'iw dev {}' ERROR: {}",
                                      device,
                                      from_utf8(&output.stderr)?)));
    }

    let iw_out = from_utf8(&output.stdout)?;
    let str2f64 = |rate_str: &str| rate_str.parse::<f64>().ok();

    let re = Regex::new(r"tx bitrate: ([^\s]+) ([^\s]+)")?;
    let bitrate = re.captures(iw_out)

    // group1 contains the bitrate
        .and_then(|captures| captures.at(1)

                  // and should be a number
                  .and_then(&str2f64)

                  // group2 contains the units
                  .and_then( |rate| captures.at(2)

                              // if Gbits translate to Mbits
                              .map( |unit| {
                                  if unit == "Gbit/s" {
                                      (rate * 1000., unit)
                                  } else {
                                      (rate, unit)
                                  }
                              })));

    let re = Regex::new(r"signal: ([-0-9]+)")?;
    let signal_dbm = re.captures(iw_out)
        .and_then(|captures| captures.at(1))
        .and_then(&str2f64);

    let re = Regex::new(r"SSID: (.+)")?;
    let ssid = re.captures(iw_out)
        .and_then(|captures| captures.at(1))
        .map(|ssid| ssid.to_string());

    let mut ip = None;
    if do_ip {
        let output = Command::new("ip").arg("addr")
            .arg("list")
            .output()?;

        if !output.status.success() {
            return Err(
                From::from(
                    format!("'ip addr list' ERROR: {}",
                            from_utf8(&output.stderr)?)));
        }

        let re = Regex::new(r"inet\s+([0-9.]+)")?;
        ip = re.captures(iw_out)
            .and_then(|captures| captures.at(1))
            .map(|ip| ip.to_string());
    }

    // reset _max_bitrate if we have changed network
    let mut max_bitrate = last_status.max_bitrate;
    if last_status.ssid != ssid {
        max_bitrate = bitrate_degraded;
    };

    // If we have a bitrate set new max. Compute quality
    let quality = match bitrate {
        Some((bits, _)) => {
            if bits > max_bitrate {
                max_bitrate = bits;
            }
            (bits / max_bitrate * 100.) as i64
        }
        None => 0,
    };

    Ok(WifiStatus {
        max_bitrate: max_bitrate,
        ssid: ssid,
        ip: ip,
        signal: signal_dbm,
        quality: quality,
    })
}
