extern crate r3bar;
#[macro_use] extern crate conrod;
extern crate getopts;

use conrod::color::{self, Color};
use getopts::Options;
use r3bar::error::BarError;
use r3bar::bar;
use r3bar::gauges::{self, icon_text};
use r3bar::sensors::{self, Sensor, i3workspace};
use r3bar::message::{Message, WebpackInfo};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use std::{env, thread};

static FONT_PATH: &'static str = "projects/r3bar/assets/fonts/Roboto Mono for Powerline.ttf";
static BATTERY_PATH: &'static str = "projects/r3bar/assets/icons/battery";
static VOLUME_PATH: &'static str = "projects/r3bar/assets/icons/volume";

const BASE03: Color = Color::Rgba(0., 0.168627, 0.211764, 1.);
const BASE02: Color = Color::Rgba(0.027450, 0.211764, 0.258823, 1.);
const BASE01: Color = Color::Rgba(0.345098, 0.431372, 0.458823, 1.);
#[allow(dead_code)]
const BASE00: Color = Color::Rgba(0.396078, 0.482352, 0.513725, 1.);
const BASE0: Color = Color::Rgba(0.513725, 0.580392, 0.588235, 1.);
#[allow(dead_code)]
const BASE1: Color = Color::Rgba(0.576470, 0.631372, 0.631372, 1.);
#[allow(dead_code)]
const CYAN: Color = Color::Rgba(0.164705, 0.631372, 0.596078, 1.);
#[allow(dead_code)]
const ORANGE: Color = Color::Rgba(0.796078, 0.294117, 0.086274, 1.);
const MAGENTA: Color = Color::Rgba(0.827450, 0.211764, 0.509803, 1.);

const BAR_HEIGHT: u32 = 26;

fn default_icon(id: conrod::image::Id) -> icon_text::Icon {
    icon_text::Icon {
        w: 24.0,
        h: 24.0,
        id: id,
        padding: 0.0,
    }
}

enum BatteryIcon {
    Charged,
    Charging,
    Empty,
    Full,
    Half,
    Low,
    None,
}

impl BatteryIcon {
    pub fn to_struct(&self, battery: &BatteryIcons) -> icon_text::Icon {
        match *self {
            BatteryIcon::Charged => battery.charged,
            BatteryIcon::Charging => battery.charging,
            BatteryIcon::Empty => battery.empty,
            BatteryIcon::Full => battery.full,
            BatteryIcon::Half => battery.half,
            BatteryIcon::Low =>  battery.low,
            BatteryIcon::None => battery.none,
        }
    }
}

struct BatteryIcons {
    charged: icon_text::Icon,
    charging: icon_text::Icon,
    empty: icon_text::Icon,
    full: icon_text::Icon,
    half: icon_text::Icon,
    low: icon_text::Icon,
    none: icon_text::Icon,
}

impl BatteryIcons {
    pub fn new<F>(path_to_id: F) -> BatteryIcons
        where F: Fn(PathBuf) -> Result<conrod::image::Id, BarError> {

        let home = env::home_dir().unwrap();
        let path = home.join(Path::new(BATTERY_PATH));
        let convert = |p| default_icon(path_to_id(path.join(p)).unwrap());

        BatteryIcons {
            charged: convert("charged-battery.png"),
            charging: convert("charging-battery.png"),
            empty: convert("empty-battery.png"),
            full: convert("full-battery.png"),
            half: convert("half-charged-battery.png"),
            low: convert("low-battery.png"),
            none: convert("no-battery.png"),
        }
    }
}

enum VolumeIcon {
    High,
    Medium,
    Low,
    Mute,
    None,
}

impl VolumeIcon {
    pub fn to_struct(&self, volume: &VolumeIcons) -> icon_text::Icon {
        match *self {
            VolumeIcon::High => volume.high,
            VolumeIcon::Medium => volume.medium,
            VolumeIcon::Low => volume.low,
            VolumeIcon::Mute => volume.mute,
            VolumeIcon::None => volume.none,
        }
    }
}

struct VolumeIcons {
    high: icon_text::Icon,
    medium: icon_text::Icon,
    low: icon_text::Icon,
    mute: icon_text::Icon,
    none: icon_text::Icon,
}

impl VolumeIcons {
    pub fn new<F>(path_to_id: F) -> VolumeIcons
        where F: Fn(PathBuf) -> Result<conrod::image::Id, BarError> {

        let home = env::home_dir().unwrap();
        let path = home.join(Path::new(VOLUME_PATH));
        let convert = |p| default_icon(path_to_id(path.join(p)).unwrap());

        VolumeIcons {
            high: convert("high-volume.png"),
            medium: convert("medium-volume.png"),
            low: convert("low-volume.png"),
            mute: convert("mute-volume.png"),
            none: convert("no-audio.png"),
        }
    }
}

struct Battery {
    capacity: f64,
    icon: BatteryIcon,
}

struct Volume {
    percent: f64,
    icon: VolumeIcon,
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
    wifi: r3bar::sensors::wifi::WifiStatus,
    diskusage: String,
}

struct Store {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Message>,
    state: Arc<Mutex<State>>,
    handles: Vec<thread::JoinHandle<Result<(), BarError>>>,
}

impl Store {
    fn update(&self, msg: Message) {

        // unwrap is intentional. If a thread panics we want bring the system down.
        let mut state = self.state.lock().unwrap();
        match msg {
            Message::Time(time) => state.time = time,

            Message::Battery((capacity, _, ac)) => {
                match capacity.parse::<f64>() {
                    Ok(cap) => {
                        state.battery.capacity = cap;
                        if ac == "1" {
                            state.battery.icon = BatteryIcon::Charging;
                        } else {
                            state.battery.icon = match cap {
                                0.0...5.0 => BatteryIcon::Empty,
                                5.0...35.0 => BatteryIcon::Low,
                                35.0...75.0 => BatteryIcon::Half,
                                75.0...95.0 => BatteryIcon::Charged,
                                95.0...100.0 => BatteryIcon::Full,
                                _ => BatteryIcon::None,
                            }
                        }
                    }

                    Err(e) => {
                        println!("Battery capacity parse error {}", e);
                        state.battery.capacity = 0.;
                        state.battery.icon = BatteryIcon::None;
                    }
                }
            }

            Message::Workspaces(workspaces) => {
                let mut work_vec = Vec::new();
                for workspace in workspaces {
                    let mut color;
                    if workspace.focused {
                        color = BASE0;
                    } else {
                        color = BASE01;
                    }
                    if workspace.urgent {
                        color = color.complement();
                    }
                    work_vec.push((workspace.name.clone(), color));
                }

                state.i3.workspaces = work_vec;
            }

            Message::I3Mode(mode) => {
                if mode == "default" {
                    state.i3.mode = "".to_string();
                } else {
                    state.i3.mode = mode;
                }
            }

            Message::Unpark => {
                for handle in self.handles.iter() {
                    handle.thread().unpark();
                }
            }

            Message::Wifi(status) => state.wifi = status,

            Message::Webpack(info) => state.webpack = info,

            Message::Volume(volume) => {
                match volume.parse::<f64>() {
                    Ok(vol) => {
                        state.volume.percent = vol;
                        state.volume.icon = match vol {
                            0.0 => VolumeIcon::Mute,
                            0.0...35.0 => VolumeIcon::Low,
                            35.0...75.0 => VolumeIcon::Medium,
                            75.0...100.0 => VolumeIcon::High,
                            _ => VolumeIcon::None,
                        }
                    }
                    Err(e) => {
                        println!("Volume parse error {}", e);
                        state.volume.percent = 0.;
                        state.volume.icon = VolumeIcon::None;
                    }
                }
            }

            Message::DiskUsage(usage) => {
                state.diskusage = usage;
            }

            Message::Error(e) => println!("Msg Error: {}", e),

            Message::Exit(code) => std::process::exit(code),
        };
    }

    fn register<S: Sensor>(&mut self, sensor: &S) {
        match sensor.run(self.tx.clone()) {
            Ok(handle) => self.handles.push(handle),
            Err(e) => {
                println!("{}", e);
            }
        }
    }

    fn listen(self, ui_txs: Vec<mpsc::Sender<bar::DispResponse>>) -> thread::JoinHandle<()>
    {

        let txs: Vec<mpsc::Sender<bar::DispResponse>> = ui_txs.iter().map(|tx| tx.clone()).collect();

        let listener = thread::spawn(move || {
            let txs = txs.clone();
            loop {

                // channels will throw an error when the other ends disconnect.
                let msg = match self.rx.recv() {
                    Err(_) => break, // LOGGING error/disconnect?
                    Ok(msg) => msg,
                };

                self.update(msg);

                for tx in txs.iter() {
                    tx.send(bar::DispResponse::WakeDisplay).unwrap();
                }
            }
        });

        return listener;
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn set_exit_timer(seconds: u64, tx: mpsc::Sender<Message>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(seconds));
        tx.send(Message::Exit(0)).unwrap();
    });
}

fn main() {

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut exit_seconds = None;

    if args.len() > 1 {
        let mut opts = Options::new();
        opts.optopt("b", "bench", "run program for n seconds", "SECONDS");
        opts.optflag("h", "help", "print this help menu");
        let matches = match opts.parse(&args[1..]) {
            Ok(m) => m,
            Err(f) => panic!(f.to_string()),
        };

        if matches.opt_present("h") {
            print_usage(&program, opts);
            return;
        }

        match matches.opt_str("b") {
            Some(seconds) => exit_seconds = Some(seconds.parse::<i32>().unwrap()),
            None => exit_seconds = None,
        }
    }

    let (tx, rx) = mpsc::channel();

    let state = Arc::new(Mutex::new(State {
        time: "".to_owned(),
        battery: Battery {
            capacity: -1.0,
            icon: BatteryIcon::None,
        },
        i3: I3 {
            mode: "".to_owned(),
            workspaces: Vec::new(),
        },
        webpack: WebpackInfo::Done,
        wifi: sensors::wifi::WifiStatus::new(53.),
        volume: Volume {
            percent: 0.,
            icon: VolumeIcon::None,
        },
        diskusage: "".to_owned()
    }));


    // set up our store and start listening
    let mut store = Store {
        rx: rx,
        tx: tx.clone(),
        state: state.clone(),
        handles: Vec::new(),
    };

    // set up the sensors
    let i3workspace = sensors::i3workspace::I3Workspace::new();
    let systime = sensors::systime::SysTime::new(Duration::from_millis(100));
    let battery = sensors::battery::Battery::new(Duration::from_millis(5000));
    let volume = sensors::volume::Volume::new(Duration::from_millis(10000));
    let ipc = sensors::ipc::Ipc::new(None).unwrap();
    let wifi = sensors::wifi::ConfigureWifi::new().unwrap().configure();
    let diskusage = sensors::diskusage::DiskUsage::new(
        Duration::from_millis(5000), vec!["/".to_owned(), "/home".to_owned()]
    );

    store.register(&volume);
    store.register(&systime);
    store.register(&ipc);
    store.register(&i3workspace);
    store.register(&battery);
    store.register(&wifi);
    store.register(&diskusage);


    // instantiate a our system
    let r3b = r3bar::bar::Bar{};

    // bind widgets to our store state and call animate_frame to
    // start the render loop. Pass in a tx channel for our thread UIs to
    // communicate on.
    let ui_txs = r3b.run(BAR_HEIGHT, tx.clone(), Arc::new(move |ui_context: &mut bar::UiLoop, app_tx: mpsc::Sender<Message>| {

        // Set up assets
        let home = env::home_dir().unwrap();
        let font_path = home.join(Path::new(FONT_PATH));
        ui_context.set_fonts(&font_path).unwrap();

        let volume_icons;
        let battery_icons;
        {
            let loader = |p| ui_context.load_image(p);
            volume_icons = VolumeIcons::new(loader);
        }

        {
            let loader = |p| ui_context.load_image(p);
            battery_icons = BatteryIcons::new(loader);
        }

        let time_widget;
        let battery_widget;
        let wifi_widget;
        let workspace_widget;
        let redkitt;
        let volume_widget;
        let diskusage_widget;
        {
            let ui = &mut ui_context.ui;

            // change the default theme.
            ui.theme.background_color = BASE03;
            ui.theme.label_color = BASE0;
            ui.theme.padding = conrod::position::Padding::none();
            ui.theme.border_color = BASE02;
            ui.theme.font_size_medium = 14;

            // set up gauges to display sensor data
            time_widget = gauges::icon_text::IconText::new(ui.widget_id_generator());
            battery_widget = gauges::icon_text::IconText::new(ui.widget_id_generator());
            wifi_widget = gauges::icon_text::IconText::new(ui.widget_id_generator());
            workspace_widget = gauges::button_row::ButtonRow::new(
                30, BASE03, MAGENTA, ui.widget_id_generator()
            );
            redkitt = gauges::redkitt::RedKitt::new(ui.widget_id_generator());
            volume_widget = gauges::icon_text::IconText::new(ui.widget_id_generator());
            diskusage_widget = gauges::icon_text::IconText::new(ui.widget_id_generator());
        }

        // TIME
        {
            let state = state.clone();

            ui_context.bind_right(
                bar::DEFAULT_GAUGE_WIDTH + 10,
                move |slot_id, mut ui_widgets, _| {

                    let state = state.lock().unwrap();

                    time_widget.render(icon_text::Opts{
                        maybe_icon: None,
                        maybe_text: Some(&state.time),
                    }, slot_id, ui_widgets);

                });
        }

        // BATTERY
        {
            let state = state.clone();

            ui_context.bind_right(
                bar::DEFAULT_GAUGE_WIDTH / 2,
                move |slot_id, mut ui_widgets, _| {

                    let state = state.lock().unwrap();
                    let battery_icon = state.battery.icon.to_struct(&battery_icons);

                    battery_widget.render(icon_text::Opts{
                        maybe_icon: Some(battery_icon),
                        maybe_text: Some(&format!("{}%", state.battery.capacity)),
                    }, slot_id, ui_widgets);
                });
        }

        // DISK USAGE
        {
            let state = state.clone();

            ui_context.bind_right(
                bar::DEFAULT_GAUGE_WIDTH,
                move |slot_id, mut ui_widgets, _| {

                    let state = state.lock().unwrap();

                    diskusage_widget.render(icon_text::Opts{
                        maybe_icon: None,
                        maybe_text: Some(&state.diskusage),
                    }, slot_id, ui_widgets);
                });
        }

        // VOLUME
        {
            let state = state.clone();

            ui_context.bind_right(
                bar::DEFAULT_GAUGE_WIDTH / 2,
                move |slot_id, mut ui_widgets, _| {

                    let state = state.lock().unwrap();
                    let volume_icon = state.volume.icon.to_struct(&volume_icons);
                    volume_widget.render(icon_text::Opts{
                        maybe_icon: Some(volume_icon),
                        maybe_text: Some(&format!("{}%", state.volume.percent)),
                    }, slot_id, ui_widgets);
                });
        }

        // WIFI
        {
            let state = state.clone();

            ui_context.bind_right(
                bar::DEFAULT_GAUGE_WIDTH,
                move |slot_id, mut ui_widgets, _| {

                    let state = state.lock().unwrap();
                    let ssid = state.wifi.ssid.clone()
                        .unwrap_or("unconnected".to_string());
                    let signal_quality = state.wifi.signal.map(dbm_to_percent)
                        .unwrap_or(0.);

                    let wifi_line = format!("{}  {}%", ssid, signal_quality);

                    wifi_widget.render(icon_text::Opts{
                        maybe_icon: None,
                        maybe_text: Some(&wifi_line),
                    }, slot_id, ui_widgets);
                });
        }

        // WEBPACK SENSOR
        {
            let state = state.clone();
            ui_context.bind_right(
                bar::DEFAULT_GAUGE_WIDTH,
                move |slot_id, mut ui_widgets, dt| {

                    let state = state.lock().unwrap();
                    let do_animate = match state.webpack {
                        WebpackInfo::Compile => true,
                        _ => false,
                    };
                    if let Some(_) = redkitt.render(
                        do_animate, slot_id, ui_widgets, dt
                    ) {
                        let done_msg = Message::Webpack(WebpackInfo::Done);
                        if let Err(e) = app_tx.send(done_msg) {
                            println!("{}", e); // logging
                        }
                    }
                });
        }

        // I3 WORKSPACES
        {
            let state = state.clone();

            ui_context.bind_left(
                bar::DEFAULT_GAUGE_WIDTH + bar::DEFAULT_GAUGE_WIDTH / 2,
                move |slot_id, mut ui_widgets, _| {

                    let state = state.lock().unwrap();
                    let maybe_clicked = workspace_widget
                        .render(
                            state.i3.workspaces.clone(),
                            &state.i3.mode,
                            slot_id,
                            ui_widgets);


                    if let Some(ith_btn) = maybe_clicked {
                        let w_index = ith_btn + 1; // 1 indexed
                        let r = i3workspace::I3Workspace::change_workspace(w_index);
                        if let Err(e) = r {
                                println!("{}", e); // TODO logging
                            }
                        }
                });
        }
    }));

    let listener = store.listen(ui_txs);

    // if there is an exit timer set it.
    if let Some(seconds) = exit_seconds {
        set_exit_timer(seconds as u64, tx.clone());
    }

    listener.join();
}

fn dbm_to_percent(dbm: f64) -> f64 {
    2. * (dbm + 100.)
}
