use i3ipc::I3Connection;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use std::sync::mpsc;
use std::thread;

use message::Message;

pub struct I3Workspace {}

impl I3Workspace {
    pub fn run(&self, tx: mpsc::Sender<Message>) {
        thread::spawn(move || {
            // establish connection.
            let mut listener = I3EventListener::connect().unwrap();

            // subscribe to a couple events.
            let subs = [Subscription::Workspace];
            listener.subscribe(&subs).unwrap();

            let mut connection = I3Connection::connect().unwrap();

            // handle them
            for event in listener.listen() {
                match event.unwrap() {
                    Event::WorkspaceEvent(_) => {
                        let workspaces = connection.get_workspaces().unwrap();
                        tx.send(Message::Workspaces(workspaces)).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        });
    }
}
