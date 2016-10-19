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
mod message;
mod bar;

use message::Message;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use i3ipc::reply::Workspace;
use conrod::{color, widget, Colorable, Positionable, Sizeable, Widget};

const FONT_SIZE: conrod::FontSize = 14;
const LINE_SPACING: f64 = 2.5;
const PAD: f64 = 20.0;

struct State {
    time: String,
    workspaces: Vec<Workspace>,
}

struct Store {
    state: Arc<Mutex<State>>,
}

impl Store {
    fn update(&self, msg: Message) {
        let mut state = self.state.lock().unwrap(); // TODO
        match msg {
            Message::Time(time) => state.time = time,
            Message::Workspaces(w) => state.workspaces = w.workspaces,
            Message::Unlisten => return,
        };
    }

    fn listen(self, rx: mpsc::Receiver<Message>) -> thread::JoinHandle<()> {
        let listener = thread::spawn(move || {
            loop {
                let msg = rx.recv().unwrap(); // TODO
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
        time: "".to_string(),
        workspaces: Vec::new()
    }));

    // set up our store and start listening
    let store = Store { state: state.clone() };
    store.listen(rx);

    // set up some sensors to produce data
    let systime = sensors::systime::SysTime{};
    systime.run(tx.clone());

    let i3workspace = sensors::i3workspace::I3Workspace{};
    i3workspace.run(tx.clone());

    // instantiate a our system bar
    let mut rubar = bar::Bar::new();

    // While rubar provides an ID for each space on the bar we need extra
    // Ids to compose the actual widget structures themselves.
    let workspace_text = rubar.new_id();
    let middle_text = rubar.new_id();

    rubar.bind_widget(move |state: &MutexGuard<State>, spacer_id, mut ui_widgets| {
        let time_str = &state.time;
        widget::Text::new(time_str)
            .color(color::LIGHT_GREEN)
            .padded_w_of(spacer_id, PAD)
            .middle_of(spacer_id)
            .align_text_middle()
            .line_spacing(LINE_SPACING)
            .font_size(FONT_SIZE)
            .set(middle_text, &mut ui_widgets);

    }).bind_widget(move |state: &MutexGuard<State>, spacer_id, mut ui_widgets| {
        let ref workspaces = state.workspaces;
        let mut active_workspace = "".to_string();
        for i in 0..workspaces.len() {
            let ref workspace = workspaces[i];
            if workspace.focused {
                active_workspace = workspace.num.to_string();
            }
        }

        widget::Text::new(&active_workspace)
            .color(color::LIGHT_GREEN)
            .padded_w_of(spacer_id, PAD)
            .middle_of(spacer_id)
            .align_text_middle()
            .line_spacing(LINE_SPACING)
            .font_size(FONT_SIZE)
            .set(workspace_text, &mut ui_widgets);

    }).animate_frame(state);
}
