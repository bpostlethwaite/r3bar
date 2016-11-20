extern crate byteorder;
extern crate chrono;
#[macro_use] extern crate conrod;
extern crate i3ipc;
extern crate piston_window;
extern crate regex;
extern crate serde_json;
extern crate unix_socket;

mod animate;
mod bar;
mod error;
mod gauges;
mod sensors;
mod message;
mod widgets;

use conrod::{Padding};
use conrod::color::{Color, self};
use error::BarError;
use message::{Message, WebpackInfo};
use sensors::wifi::WifiStatus;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::time::Duration;
use std::{env, thread};


static FONT_PATH: &'static str = "programming/rubar/assets/fonts/Roboto Mono for Powerline.ttf";

const BASE03: Color = Color::Rgba(0., 0.168627, 0.211764, 1.);
const BASE02: Color = Color::Rgba(0.027450, 0.211764, 0.258823, 1.);
const BASE01: Color = Color::Rgba(0.345098, 0.431372, 0.458823, 1.);
const BASE00: Color = Color::Rgba(0.396078, 0.482352, 0.513725, 1.);
const BASE0: Color = Color::Rgba(0.513725, 0.580392, 0.588235, 1.);
const BASE1: Color = Color::Rgba(0.576470, 0.631372, 0.631372, 1.);
const CYAN: Color = Color::Rgba(0.164705, 0.631372, 0.596078, 1.);
const ORANGE: Color = Color::Rgba(0.796078, 0.294117, 0.086274, 1.);
const MAGENTA: Color = Color::Rgba(0.827450, 0.211764, 0.509803, 1.);

const HEIGHT: u32 = 26;

struct Battery {
    capacity: String,
    status: String,
}

struct I3 {
    mode: String,
    workspaces: Vec<(String, color::Color)>,
}

struct State {
    battery: Battery,
    time: String,
    i3: I3,
    webpack: WebpackInfo,
    wifi: WifiStatus,
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

                state.i3.workspaces = work_vec;
            },

            Message::I3Mode(mode) => {
                if mode == "default" {
                    state.i3.mode = "".to_string();
                } else {
                    state.i3.mode = mode;
                }
            },

            Message::Wifi(status) => state.wifi = status,

            Message::Webpack(info) => state.webpack = info,

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

fn error_exit(error: error::BarError) {
    println!("{}", error);
    std::process::exit(1);
}

fn main() {

    let (tx, rx) = mpsc::channel();

    let state = Arc::new(Mutex::new(State {
        time: "".to_string(),
        battery: Battery{
            capacity: "".to_string(),
            status: "".to_string(),
        },
        i3: I3{
            mode: "".to_string(),
            workspaces: Vec::new(),
        },
        webpack: WebpackInfo::Done,
        wifi: WifiStatus::new(53.),
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
            error_exit(e);
        }

    // change the default theme.
    rubar.ui.theme.background_color = BASE03;
    rubar.ui.theme.label_color = BASE0;
    rubar.ui.theme.padding = Padding::none();
    rubar.ui.theme.border_color = BASE02;
    rubar.ui.theme.font_size_medium = 14;

    // set up the sensors
    let i3workspace = sensors::i3workspace::I3Workspace::new();
    let systime = sensors::systime::SysTime::new(Duration::from_millis(100));
    let battery = sensors::battery::Battery::new(Duration::from_millis(5000));
    let ipc = sensors::ipc::Ipc::new();

    // run the sensors and collect any errors
    if let Err(e) = || -> Result<(), BarError> {
        ipc.run(tx.clone())?;
        systime.run(tx.clone(), Message::Time)?;
        i3workspace.run(tx.clone(), Message::Workspaces, Message::I3Mode)?;
        battery.run(tx.clone(), Message::Battery)?;
        sensors::wifi::ConfigureWifi::new()?.configure()
            .run(tx.clone(), Message::Wifi)?;
        Ok(())
    }() {
        // if any sensors fail bail.
        error_exit(e);
    }

    // set up gauges to display sensor data
    let time_widget = gauges::simple_text::Simple::new(
        rubar.ui.widget_id_generator());

    let battery_widget = gauges::simple_text::Simple::new(
        rubar.ui.widget_id_generator());

    let wifi_widget = gauges::simple_text::Simple::new(
        rubar.ui.widget_id_generator());

    let workspace_widget = gauges::button_row::ButtonRow::new(
        30, BASE03, MAGENTA, rubar.ui.widget_id_generator()
    );

    let redkitt = gauges::redkitt::RedKitt::new(
        rubar.ui.widget_id_generator());

    // bind widgets to our store state and call animate_frame to
    // start the render loop.
    rubar
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH + 10,
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
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                let ssid = state.wifi.ssid.clone()
                    .unwrap_or("unconnected".to_string());
                let signal_quality = state.wifi.signal.map(dbm_to_percent)
                    .unwrap_or(0.);

                let wifi_line = format!("{}  {}%", ssid, signal_quality);

                wifi_widget.render(&wifi_line, slot_id, ui_widgets);
            })
        .bind_left(
            bar::DEFAULT_GAUGE_WIDTH + bar::DEFAULT_GAUGE_WIDTH / 2,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                if let Some(ibtn) = workspace_widget
                    .render(
                        state.i3.workspaces.clone(),
                        &state.i3.mode,
                        slot_id,
                        ui_widgets) {

                        if let Err(e) = i3workspace.change_workspace(ibtn + 1) {
                            println!("{}", e); // logging
                        }
                    }
            })
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                let do_animate = match state.webpack {
                    WebpackInfo::Compile => true,
                    _ => false,
                };

                if let Some(_) = redkitt.render(do_animate, slot_id, ui_widgets) {
                    if let Err(e) = tx.send(Message::Webpack(WebpackInfo::Done)) {
                        println!("{}", e); // logging
                    }
                }
            })
        .animate_frame(state);
}


fn dbm_to_percent(dbm: f64) -> f64 {
    2. * (dbm + 100.)
}

fn percent_to_dbm(percent: f64) -> f64 {
    (percent / 2.) - 100.
}
