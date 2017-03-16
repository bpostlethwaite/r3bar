use i3ipc::I3Connection;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use message::Message;
use error::BarError;
use sensors::{Sensor, SensorResult};
use std::sync::mpsc::{Sender};
use std::thread;

pub struct I3Workspace {}

impl I3Workspace {
    pub fn new() -> I3Workspace {
        I3Workspace{}
    }

    pub fn change_workspace(workspace: String, output: String) -> Result<(), BarError> {
        let cmd = format!("workspace {}, move workspace to {}", workspace, output);

        let mut connection = I3Connection::connect()?;
        let outcomes = connection.command(&cmd).ok().expect("failed to send command").outcomes;

        for outcome in outcomes {
            if !outcome.success {
                match outcome.error {
                    Some(e) => return Err(From::from(e)),
                    None => return Err(BarError::Bar("Couldn't switch workspace unknown reason".to_owned())),
                }
            }
        }

        Ok(())
    }

}

impl Sensor for I3Workspace {
    fn run(&self, tx: Sender<Message>) -> SensorResult {

        // send a snapshot of current workspace immediately
        let mut connection = I3Connection::connect()?;
        let w = connection.get_workspaces()?;
        tx.send(Message::Workspaces(w.workspaces)).map_err(|e| e.to_string())?;

        Ok(thread::spawn(move || {

            // only ask for all workspaces when we detect a related event
            let subs = [Subscription::Workspace, Subscription::Mode, Subscription::Window];
            let mut listener = I3EventListener::connect().unwrap();
            listener.subscribe(&subs).unwrap();

            for event in listener.listen() {
                match event {
                    Ok(Event::WorkspaceEvent(_)) | Ok(Event::WindowEvent(_)) => {
                        let w = connection.get_workspaces().unwrap();
                        tx.send(Message::Workspaces(w.workspaces)).unwrap();
                    }
                    Ok(Event::ModeEvent(e)) => {
                        tx.send(Message::I3Mode(e.change)).unwrap();
                    }
                    _ => println!("bad things from i3workspace"),
                }
            }
            Ok(())
        }))
    }
}
