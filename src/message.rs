use r3bar::error::BarError;
use r3bar::sensors::ipc::R3Msg;
use i3ipc::reply::Workspace;
use r3bar::sensors::wifi::WifiStatus;
use r3bar::r3ipc;
use serde_json as json;
use std::str::FromStr;


#[derive(Debug)]
pub enum Message {
    Battery((String, String, String)),
    I3Mode(String),
    Error(String),
    Time(String),
    Unpark,
    Volume(String),
    Webpack(WebpackInfo),
    Wifi(WifiStatus),
    Workspaces(Vec<Workspace>),
}

impl From<R3Msg> for Message {
    fn from(r3msg: R3Msg) -> Message {

        match || -> Result<Message, BarError> {
            Ok(match r3msg.msgtype {
                c @ 0...r3ipc::RESERVED => Message::Error(
                    format!("R3Msg: reserved code range {}", c)),
                r3ipc::WEBPACK => Message::Webpack(WebpackInfo::from_str(&r3msg.payload)?),
                r3ipc::UNPARK => Message::Unpark,
                _ => Message::Error(
                    format!("R3Msg: msgtype '{}' not implemented", r3msg.msgtype)),
            })
        }() {
            Ok(message) => message,
            Err(errmsg) => Message::Error(errmsg.to_string()),
        }
    }
}


#[derive(Debug, Clone)]
pub enum WebpackInfo {
    Compile,
    Done,
}

/// Data for `WebpackEvent`.
impl FromStr for WebpackInfo {
    type Err = BarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data: json::Value = json::from_str(s)?;
        let obj = data.as_object()
            .ok_or(BarError::Bar(format!("webpack json not object")))?;

        let val = obj.get("change").unwrap();
        let change = match *val {
            json::Value::String(ref v) => v,
            _ => Err(BarError::Bar(format!("change field missing String")))?,
        };

        Ok(match change.as_ref() {
            "compile" => WebpackInfo::Compile,
            "done" => WebpackInfo::Done,
            _ => unreachable!(),
        })
    }
}
