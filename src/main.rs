extern crate byteorder;
extern crate chrono;
#[macro_use] extern crate conrod;
extern crate i3ipc;
extern crate piston_window;
extern crate regex;
extern crate serde_json;
extern crate unix_socket;
extern crate gfx_device_gl;

mod animate;
mod bar;
mod error;
mod gauges;
mod sensors;
mod message;
mod widgets;

use conrod::color::{Color, self};
use conrod::widget::{Id};
use error::BarError;
use gauges::icon_text;
use message::{Message, WebpackInfo};
use sensors::wifi::WifiStatus;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::time::Duration;
use std::{env, thread};


static FONT_PATH: &'static str = "programming/rubar/assets/fonts/Roboto Mono for Powerline.ttf";

static BATTERY_PATH: &'static str = "programming/rubar/assets/icons/battery";
static VOLUME_PATH: &'static str = "programming/rubar/assets/icons/volume";

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

struct BatteryIcons {
    charged: icon_text::Icon,
    charging: icon_text::Icon,
    empty: icon_text::Icon,
    full: icon_text::Icon,
    half: icon_text::Icon,
    low: icon_text::Icon,
    none: icon_text::Icon,
}

struct VolumeIcons{
    high: icon_text::Icon,
    medium: icon_text::Icon,
    low: icon_text::Icon,
    mute: icon_text::Icon,
    none: icon_text::Icon,
}

struct Battery {
    capacity: f64,
    icon: icon_text::Icon,
}

struct Volume {
    percent: f64,
    icon: icon_text::Icon,
}

struct I3 {
    mode: String,
    workspaces: Vec<(String, color::Color)>,
}

struct State {
    battery: Battery,
    time: String,
    volume: Volume,
    i3: I3,
    webpack: WebpackInfo,
    wifi: WifiStatus,
}

struct Store<T> {
    state: Arc<Mutex<State>>,
    handles: Vec<thread::JoinHandle<T>>,
    battery_icons: BatteryIcons,
    volume_icons: VolumeIcons,
}


impl <T: 'static + Send>Store<T> {
    fn update(&self, msg: Message) {

        // unwrap is intentional. If a thread panics we want bring the system down.
        let mut state = self.state.lock().unwrap();
        match msg {
            Message::Time(time) => state.time = time,

            Message::Battery( (capacity, _, ac) ) => {

                match capacity.parse::<f64>() {
                    Ok(cap) => {
                        state.battery.capacity = cap;
                        if ac == "1" {
                            state.battery.icon = self.battery_icons.charging;
                        } else {
                            state.battery.icon = match cap {
                                0.0 ... 5.0 => self.battery_icons.empty,
                                5.0 ... 35.0 => self.battery_icons.low,
                                35.0 ... 75.0 => self.battery_icons.half,
                                75.0 ... 95.0 => self.battery_icons.charged,
                                95.0 ... 100.0 => self.battery_icons.full,
                                _ => self.battery_icons.none,
                            }
                        }
                    },

                    Err(e) => {
                        println!("Battery capacity parse error {}", e);
                        state.battery.capacity = 0.;
                        state.battery.icon = self.battery_icons.none;
                    }
                }
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

            Message::Unpark => for handle in self.handles.iter() {
                handle.thread().unpark();
            },

            Message::Wifi(status) => state.wifi = status,

            Message::Webpack(info) => state.webpack = info,

            Message::Volume(volume) => {
                match volume.parse::<f64>() {
                    Ok(vol) => {
                        state.volume.percent = vol;
                        state.volume.icon = match vol {
                            0.0 => self.volume_icons.mute,
                            0.0 ... 35.0 => self.volume_icons.low,
                            35.0 ... 75.0 => self.volume_icons.medium,
                            75.0 ... 100.0 => self.volume_icons.high,
                            _ => self.volume_icons.none,
                        }
                    },
                    Err(e) => {
                        println!("Volume parse error {}", e);
                        state.volume.percent = 0.;
                        state.volume.icon = self.volume_icons.none;
                    }
                }
            },

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

    // instantiate a our system
    let mut rubar = bar::Bar::new(HEIGHT);

    // load up some assets
    let home = env::home_dir().unwrap();
    let font_path = home.join(Path::new(FONT_PATH));
    let bat_path = home.join(Path::new(BATTERY_PATH));
    let vol_path = home.join(Path::new(VOLUME_PATH));

    rubar.set_fonts(&font_path).unwrap();

    let bpath = |p| bat_path.join(p);
    let vpath = |p| vol_path.join(p);
    let ic = |id| icon_text::Icon{w: 24.0, h: 24.0, id: id, padding: 0.0};

    let battery_icons = BatteryIcons{
        charged: ic(rubar.load_icons(&bpath("charged-battery.png")).unwrap()),
        charging: ic(rubar.load_icons(&bpath("charging-battery.png")).unwrap()),
        empty: ic(rubar.load_icons(&bpath("empty-battery.png")).unwrap()),
        full: ic(rubar.load_icons(&bpath("full-battery.png")).unwrap()),
        half: ic(rubar.load_icons(&bpath("half-charged-battery.png")).unwrap()),
        low: ic(rubar.load_icons(&bpath("low-battery.png")).unwrap()),
        none: ic(rubar.load_icons(&bpath("no-battery.png")).unwrap()),
    };

    let volume_icons = VolumeIcons{
        high: ic(rubar.load_icons(&vpath("high-volume.png")).unwrap()),
        medium: ic(rubar.load_icons(&vpath("medium-volume.png")).unwrap()),
        low: ic(rubar.load_icons(&vpath("low-volume.png")).unwrap()),
        mute: ic(rubar.load_icons(&vpath("mute-volume.png")).unwrap()),
        none: ic(rubar.load_icons(&vpath("no-audio.png")).unwrap()),
    };

    // change the default theme.
    rubar.ui.theme.background_color = BASE03;
    rubar.ui.theme.label_color = BASE0;
    rubar.ui.theme.padding = conrod::Padding::none();
    rubar.ui.theme.border_color = BASE02;
    rubar.ui.theme.font_size_medium = 14;

    // set up the sensors
    let i3workspace = sensors::i3workspace::I3Workspace::new();
    let systime = sensors::systime::SysTime::new(Duration::from_millis(100));
    let battery = sensors::battery::Battery::new(Duration::from_millis(5000));
    let volume = sensors::volume::Volume::new(Duration::from_millis(10000));
    let ipc = sensors::ipc::Ipc::new();

    // Bind sensors to the Message enum
    if let Err(e) = || -> Result<(), BarError> {
        ipc.run(tx.clone())?;
        systime.run(tx.clone(), Message::Time)?;
        i3workspace.run(tx.clone(), Message::Workspaces, Message::I3Mode)?;
        battery.run(tx.clone(), Message::Battery)?;
        sensors::wifi::ConfigureWifi::new()?.configure()
            .run(tx.clone(), Message::Wifi)?;
        volume.run(tx.clone(), Message::Volume)?;
        Ok(())
    }() {
        // if any sensors fail bail.
        error_exit(e);
    }

    let thread = match volume.run(tx.clone(), Message::Volume) {
        Err(e) => return (),
        Ok(thread) => thread,
    };

    // set up gauges to display sensor data
    let time_widget = gauges::icon_text::IconText::new(
        rubar.ui.widget_id_generator());

    let battery_widget = gauges::icon_text::IconText::new(
        rubar.ui.widget_id_generator());

    let wifi_widget = gauges::icon_text::IconText::new(
        rubar.ui.widget_id_generator());

    let workspace_widget = gauges::button_row::ButtonRow::new(
        30, BASE03, MAGENTA, rubar.ui.widget_id_generator()
    );

    let redkitt = gauges::redkitt::RedKitt::new(
        rubar.ui.widget_id_generator());

    let volume_widget = gauges::icon_text::IconText::new(
        rubar.ui.widget_id_generator());


    let state = Arc::new(Mutex::new(State {
        time: "".to_string(),
        battery: Battery{
            capacity: -1.0,
            icon: battery_icons.none,
        },
        i3: I3{
            mode: "".to_string(),
            workspaces: Vec::new(),
        },
        webpack: WebpackInfo::Done,
        wifi: WifiStatus::new(53.),
        volume: Volume{
            percent: 0.,
            icon: volume_icons.none,
        },
    }));

    // set up our store and start listening
    let store = Store {
        state: state.clone(),
        handles: vec![thread],
        battery_icons: battery_icons,
        volume_icons: volume_icons,
    };
    store.listen(rx);

    // bind widgets to our store state and call animate_frame to
    // start the render loop.
    rubar

    // TIME
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH + 10,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                time_widget.render(icon_text::Opts{
                    maybe_icon: None,
                    maybe_text: Some(&state.time),
                }, slot_id, ui_widgets);

            })

    // BATTERY
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH / 2,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {
                battery_widget.render(icon_text::Opts{
                    maybe_icon: Some(state.battery.icon),
                    maybe_text: Some(&format!("{}%", state.battery.capacity)),
                }, slot_id, ui_widgets);
            })

    // VOLUME
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH / 2,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                volume_widget.render(icon_text::Opts{
                    maybe_icon: Some(state.volume.icon),
                    maybe_text: Some(&format!("{}%", state.volume.percent)),
                }, slot_id, ui_widgets);

            })

    // WIFI
        .bind_right(
            bar::DEFAULT_GAUGE_WIDTH,
            move |state: &MutexGuard<State>, slot_id, mut ui_widgets| {

                let ssid = state.wifi.ssid.clone()
                    .unwrap_or("unconnected".to_string());
                let signal_quality = state.wifi.signal.map(dbm_to_percent)
                    .unwrap_or(0.);

                let wifi_line = format!("{}  {}%", ssid, signal_quality);

                wifi_widget.render(icon_text::Opts{
                    maybe_icon: None,
                    maybe_text: Some(&wifi_line),
                }, slot_id, ui_widgets);
            })

    // WEBPACK SENSOR
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

    // I3 WORKSPACES
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
        .animate_frame(state);
}


fn dbm_to_percent(dbm: f64) -> f64 {
    2. * (dbm + 100.)
}

fn percent_to_dbm(percent: f64) -> f64 {
    (percent / 2.) - 100.
}
