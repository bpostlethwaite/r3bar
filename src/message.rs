use error::BarError;
use i3ipc::reply::Workspace;
use sensors::wifi::WifiStatus;
use serde_json as json;
use std::str::FromStr;


#[derive(Debug)]
pub enum Message {
    Battery((String, String, String)),
    Error(BarError),
    Exit(i32),
    I3Mode(String),
    Time(String),
    Unpark,
    DiskUsage(String),
    Volume(String),
    Webpack(WebpackInfo),
    Wifi(WifiStatus),
    Workspaces(Vec<Workspace>),
}

#[derive(Debug)]
pub enum WebpackInfo {
    Compile,
    Done,
}

// Data for `WebpackEvent`.
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
