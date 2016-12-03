use i3ipc::I3Connection;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::reply::Workspace;
use std::error::Error;
use std::sync::mpsc::{Sender};
use std::{thread};

pub struct I3Workspace {}

type RunResult = Result<(), Box<Error>>;

impl I3Workspace {

    pub fn new() -> I3Workspace {
        I3Workspace{}
    }

    pub fn run<T, F, G>(&self, tx: Sender<T>, f: F, g: G) -> RunResult
        where F: 'static + Send + Fn(Vec<Workspace>) -> T,
              T: 'static + Send,
              G: 'static + Send + Fn(String) -> T
    {

        // send a snapshot of current workspace immediately
        let mut connection = try!(I3Connection::connect());
        let w = try!(connection.get_workspaces());
        try!(tx.send(f(w.workspaces)).map_err(|e| e.to_string()));

        thread::spawn(move || {

            // only ask for all workspaces when we detect a related event
            let subs = [Subscription::Workspace, Subscription::Mode];
            let mut listener = I3EventListener::connect().unwrap();
            listener.subscribe(&subs).unwrap();

            for event in listener.listen() {
                match event.unwrap() {
                    Event::WorkspaceEvent(_) => {
                        let w = connection.get_workspaces().unwrap();
                        tx.send(f(w.workspaces)).unwrap();
                    }
                    Event::ModeEvent(e) => {
                        tx.send(g(e.change)).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        });

        Ok(())
    }

    pub fn change_workspace(&self, workspace_number: i64) -> RunResult {
        let cmd = format!("workspace {}", workspace_number);

        let mut connection = try!(I3Connection::connect());
        let outcomes = connection.command(&cmd).ok().expect("failed to send command").outcomes;

        for outcome in outcomes {
            if !outcome.success {
                match outcome.error {
                    Some(e) => return Err(From::from(e)),
                    None => return Err(From::from("Couldn't switch workspace unknown reason")),
                }
            }
        }

        Ok(())
    }
}
