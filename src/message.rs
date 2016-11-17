use i3ipc::reply::{Workspace};
use sensors::wifi::WifiStatus;

#[derive(Debug)]
pub enum Message {
    Battery((String, String)),
    I3Mode(String),
    Time(String),
    Unlisten,
    Wifi(WifiStatus),
    Webpack(WebpackInfo),
    Workspaces(Vec<Workspace>),
}

#[derive(Debug, Clone)]
pub enum WebpackInfo {
    Compile,
    Done,
}
