// see _NET_WM_STRUT_PARTIAL in https://specifications.freedesktop.org/wm-spec/wm-spec-1.3.html#idm140130317566832
// and usage in https://github.com/Lokaltog/candybar/blob/develop/src/candybar.c
// and https://github.com/LemonBoy/bar/blob/master/lemonbar.c
// and very helpful see:
// http://stackoverflow.com/questions/27927433/position-toolbar-on-reserved-desktop-space-obtained-with-net-wm-strut-and-net

extern crate chrono;
extern crate conrod;
extern crate piston_window;
extern crate i3ipc;

mod sensors;
mod bar;
mod gauges;
mod error;

use error::BarError;
use i3ipc::reply::{Workspace};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::time::Duration;
use std::{env, thread};

use conrod::color::{Color, self};
use conrod::Padding;

static FONT_PATH: &'static str = "programming/rubar/assets/fonts/Roboto Mono for Powerline.ttf";

const BASE03: Color = Color::Rgba(0., 0.168627, 0.211764, 1.);
const BASE02: Color = Color::Rgba(0.027450, 0.211764, 0.258823, 1.);
const BASE01: Color = Color::Rgba(0.345098, 0.431372, 0.458823, 1.);
const BASE00: Color = Color::Rgba(0.396078, 0.482352, 0.513725, 1.);
const BASE0: Color = Color::Rgba(0.513725, 0.580392, 0.588235, 1.);
const BASE1: Color = Color::Rgba(0.576470, 0.631372, 0.631372, 1.);
const CYAN: Color = Color::Rgba(0.164705, 0.631372, 0.596078, 1.);
const ORANGE: Color = Color::Rgba(0.796078, 0.294117, 0.086274, 1.);

const HEIGHT: u32 = 26;

enum Message {
    Battery((String, String)),
    I3Mode(String),
    Time(String),
    Unlisten,
    Workspaces(Vec<Workspace>),
}

struct Battery {
    capacity: String,
    status: String,
}

struct State {
    battery: Battery,
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

            Message::Battery( (capacity, status) ) => {
                state.battery.status = status;
                state.battery.capacity = capacity;
            },

            Message::Workspaces(workspaces) => {
                let mut work_vec = Vec::new();
                for workspace in workspaces {
                    if workspace.focused {
                        work_vec.push((workspace.name.clone(), BASE0));
                    } else {
                        work_vec.push((workspace.name.clone(), BASE01));
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
        workspaces: Vec::new(),
        battery: Battery{
            capacity: "".to_string(),
            status: "".to_string(),
        }
    }));

    // set up our store and start listening
    let store = Store { state: state.clone() };
    store.listen(rx);

    // instantiate a our system
    let mut rubar = bar::Bar::new(HEIGHT);

    // load up some assets
    if let Err(e) = env::home_dir()
        .ok_or(BarError::Bar(format!("could not located HOME env")))
        .and_then( |path| {
            let font_path = path.join(Path::new(FONT_PATH));
            rubar.set_fonts(&font_path)
        }) {
            println!("{}", e);
        }

    // change the default theme.
    rubar.ui.theme.background_color = BASE03;
    rubar.ui.theme.label_color = BASE0;
    rubar.ui.theme.padding = Padding::none();
    rubar.ui.theme.border_color = BASE02;

    // set up some sensors to produce data
    let systime = sensors::systime::SysTime{
        interval: Duration::from_millis(100),
    };
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

    let battery = sensors::battery::Battery{
        interval: Duration::from_millis(5000),
    };

    if let Err(e) = battery.run(tx.clone(), Message::Battery) {
        println!("{}", e); // TODO logging
        return;
    }

    // set up some widgets
    let time_widget = gauges::simple_text::Simple::new(
        rubar.ui.widget_id_generator());

    let battery_widget = gauges::simple_text::Simple::new(
        rubar.ui.widget_id_generator());

    let workspace_widget = gauges::button_row::ButtonRow::new(
        30, BASE03, rubar.ui.widget_id_generator()
    );

    // bind widgets to our store state and finally call animate_frame to
    // start the draw render loop.
    rubar
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                time_widget.render(&state.time, slot_id, ui_widgets);

            })
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                let battery_line = format!(
                    "{}%  {}",
                    state.battery.capacity.trim(),
                    state.battery.status.trim(),
                );

                battery_widget.render(&battery_line, slot_id, ui_widgets);
            })
        .bind_left(
            bar::DEFAULT_GAUGE_WIDTH + bar::DEFAULT_GAUGE_WIDTH / 2,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                if let Some(button_number) = workspace_widget
                    .render(state.workspaces.clone(), slot_id, ui_widgets) {

                        if let Err(e) = i3workspace.change_workspace(button_number + 1) {
                            println!("{:?}", e); // logging
                        }
                    }

            })
        .animate_frame(state);
}
