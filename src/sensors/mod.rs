use error::BarError;
use message::Message;
use std::sync::mpsc;
use std::thread;

pub mod systime;
pub mod i3workspace;
pub mod battery;
pub mod wifi;
pub mod ipc;
pub mod volume;

type SensorResult = Result<thread::JoinHandle<Result<(), BarError>>, BarError>;

pub trait Sensor {
    fn run(&self, tx: mpsc::Sender<Message>) -> SensorResult;
}
