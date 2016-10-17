use i3ipc::reply::Workspaces;

pub enum Message {
    Time(String),
    Workspaces(Workspaces),
    Unlisten,
}
