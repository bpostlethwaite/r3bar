// see _NET_WM_STRUT_PARTIAL in https://specifications.freedesktop.org/wm-spec/wm-spec-1.3.html#idm140130317566832
// and usage in https://github.com/Lokaltog/candybar/blob/develop/src/candybar.c
// and https://github.com/LemonBoy/bar/blob/master/lemonbar.c
// and very helpful see:
// http://stackoverflow.com/questions/27927433/position-toolbar-on-reserved-desktop-space-obtained-with-net-wm-strut-and-net

extern crate chrono;
#[macro_use]
extern crate conrod;
extern crate find_folder;
extern crate piston_window;
extern crate i3ipc;

mod sensors;
mod message;
mod bar;

use message::Message;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use i3ipc::reply::Workspace;
use conrod::{color, widget, Colorable, Positionable, Sizeable, Widget};

// Generate a unique const `WidgetId` for each widget.
widget_ids!{
    struct Ids {
        master,
        middle_col,
        middle_text,
    }
}

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

    let store = Store { state: state.clone() };

    let systime = sensors::systime::SysTime{};
    systime.run(tx.clone());

    let i3workspace = sensors::i3workspace::I3Workspace{};
    i3workspace.run(tx.clone());

    store.listen(rx);

    let mut rubar = bar::Bar::new();

    // A unique identifier for each widget.
    let ids = Ids::new(rubar.ui.widget_id_generator());

    rubar.animate_frame(|ref mut ui_widgets| {

        // Our `Canvas` tree, upon which we will place our text widgets.
        widget::Canvas::new().flow_right(&[
            (ids.middle_col, widget::Canvas::new().color(color::DARK_CHARCOAL)),
        ]).set(ids.master, ui_widgets);

        let state = state.lock().unwrap();
        let time_str = &state.time;

        widget::Text::new(time_str)
            .color(color::LIGHT_GREEN)
            .padded_w_of(ids.middle_col, PAD)
            .middle_of(ids.middle_col)
            .align_text_middle()
            .line_spacing(LINE_SPACING)
            .font_size(FONT_SIZE)
            .set(ids.middle_text, ui_widgets);
    });
}
