// see _NET_WM_STRUT_PARTIAL in https://specifications.freedesktop.org/wm-spec/wm-spec-1.3.html#idm140130317566832
// and usage in https://github.com/Lokaltog/candybar/blob/develop/src/candybar.c
// and https://github.com/LemonBoy/bar/blob/master/lemonbar.c
// and very helpful see:
// http://stackoverflow.com/questions/27927433/position-toolbar-on-reserved-desktop-space-obtained-with-net-wm-strut-and-net

extern crate chrono;
extern crate conrod;
extern crate find_folder;
extern crate piston_window;
extern crate i3ipc;

mod sensors;
mod bar;
mod gauges;

use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use i3ipc::reply::{Workspace};

use conrod::color;

enum Message {
    Time(String),
    Workspaces(Vec<Workspace>),
    I3Mode(String),
    Unlisten,
}

struct State {
    time: String,
    i3mode: String,
    workspaces: Vec<(String, color::Color)>,
}

struct Store {
    state: Arc<Mutex<State>>,
}


impl Store {
    fn update(&self, msg: Message) {

        // unwrap is intentional. If a thread panics we want bring the system down.
        let mut state = self.state.lock().unwrap();
        match msg {
            Message::Time(time) => state.time = time,

            Message::Workspaces(workspaces) => {
                let mut work_vec = Vec::new();
                for workspace in workspaces {
                    if workspace.focused {
                        work_vec.push((workspace.name.clone(), color::LIGHT_GREEN));
                    } else {
                        work_vec.push((workspace.name.clone(), color::GRAY));
                    }
                }

                state.workspaces = work_vec;
            },

            Message::I3Mode(mode) => state.i3mode = mode,
            Message::Unlisten => return,
        };
    }

    fn listen(self, rx: mpsc::Receiver<Message>) -> thread::JoinHandle<()> {
        let listener = thread::spawn(move || {
            loop {

                // channels will throw an error when the other ends disconnect.
                let msg = match rx.recv() {
                    Err(_) => break, // LOGGING error/disconnect?
                    Ok(msg) => msg,
                };

                if let Message::Unlisten = msg {
                    break;
                }

                self.update(msg);
            }
        });

        return listener;
    }
}


fn main() {

    let (tx, rx) = mpsc::channel();

    let state = Arc::new(Mutex::new(State {
        i3mode: "default".to_string(),
        time: "".to_string(),
        workspaces: Vec::new()
    }));

    // set up our store and start listening
    let store = Store { state: state.clone() };
    store.listen(rx);

    // instantiate a our system bar
    let mut rubar = bar::Bar::new();


    // set up some sensors to produce data
    let systime = sensors::systime::SysTime{};
    if let Err(e) = systime.run(tx.clone(), Message::Time) {
        println!("{}", e); // TODO logging
        return;
    }

    let i3workspace = sensors::i3workspace::I3Workspace{};
    if let Err(e) = i3workspace.run(
        tx.clone(), Message::Workspaces, Message::I3Mode
    ) {
        println!("{}", e); // TODO logging
        return;
    }


    // set up some widgets
    let time_widget = gauges::simple_text::Simple::new(
        rubar.ui.widget_id_generator());

    let time_widget2 = gauges::simple_text::Simple::new(
        rubar.ui.widget_id_generator());


    let workspace_widget = gauges::button_row::ButtonRow::new(
        rubar.ui.widget_id_generator());

    // bind widgets to our store state and finally call animate_frame to
    // start the draw render loop.
    rubar
        .bind_right(
        move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

            time_widget.render(&state.time, slot_id, ui_widgets);

        }).bind_right(
        move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

            time_widget2.render(&state.time, slot_id, ui_widgets);

        }).bind_left(
        move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

            if let Some(button_number) = workspace_widget
                .render(state.workspaces.clone(), slot_id, ui_widgets) {

                    if let Err(e) = i3workspace.change_workspace(button_number + 1) {
                        println!("{:?}", e); // logging
                    }
                }

        }).animate_frame(state);
}
