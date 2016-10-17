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

use chrono::Local;
use message::Message;
use piston_window::{EventLoop, PistonWindow, UpdateEvent, WindowSettings};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use i3ipc::reply::Workspace;


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

    // this should be inside a builder constructor
    let dt = Local::now();
    let time_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();


    let state = Arc::new(Mutex::new(State {
        time: time_str,
        workspaces: Vec::new()
    }));

    let ui_state = state.clone();
    let store = Store { state: state };

    let systime = sensors::systime::SysTime{};
    systime.run(tx.clone());

    let i3workspace = sensors::i3workspace::I3Workspace{};
    i3workspace.run(tx.clone());

    const WIDTH: u32 = 200; // this is overridden to be screen width
    const HEIGHT: u32 = 30;

    // Construct the window.
    let mut window: PistonWindow =
        WindowSettings::new("RBAR", [WIDTH, HEIGHT])
        .opengl(piston_window::OpenGL::V3_2)
        .decorated(false)
        .exit_on_esc(true).samples(4).vsync(true).build().unwrap();
    window.set_ups(60);

    // Construct our `Ui`.
    let mut ui = conrod::UiBuilder::new().build();

    // A unique identifier for each widget.
    let ids = Ids::new(ui.widget_id_generator());

    // Add a `Font` to the `Ui`'s `font::Map` from file.
    let assets = find_folder::Search::KidsThenParents(3, 5).for_folder("assets").unwrap();
    let font_path = assets.join("fonts/Roboto Mono for Powerline.ttf");
    ui.fonts.insert_from_file(font_path).unwrap();

    let mut text_texture_cache =
        conrod::backend::piston_window::GlyphCache::new(&mut window, WIDTH, HEIGHT);

    // The image map describing each of our widget->image mappings
    // (in our case, none).
    let image_map = conrod::image::Map::new();

    // Poll events from the window.
    while let Some(event) = window.next() {

        // Convert the piston event to a conrod event.
        if let Some(e) = conrod::backend::piston_window::convert_event(event.clone(), &window) {
            ui.handle_event(e);
        }

        event.update(|_| set_ui(ui.set_widgets(), &ids, ui_state.clone()));

        window.draw_2d(&event, |c, g| {
            // Only re-draw if there was some change in the `Ui`.
            if let Some(primitives) = ui.draw_if_changed() {
                fn texture_from_image<T>(img: &T) -> &T { img };
                conrod::backend::piston_window::draw(c, g, primitives,
                                                     &mut text_texture_cache,
                                                     &image_map,
                                                     texture_from_image);
            }
        });
    }

}

fn set_ui(ref mut ui: conrod::UiCell, ids: &Ids, state: Arc<Mutex<State>>) {
    use conrod::{color, widget, Colorable, Positionable, Scalar, Sizeable, Widget};

    // Our `Canvas` tree, upon which we will place our text widgets.
    widget::Canvas::new().flow_right(&[
        (ids.middle_col, widget::Canvas::new().color(color::DARK_CHARCOAL)),
    ]).set(ids.master, ui);

    let state = state.lock().unwrap();
    let time_str = &state.time;

    widget::Text::new(time_str)
        .color(color::LIGHT_GREEN)
        .padded_w_of(ids.middle_col, PAD)
        .middle_of(ids.middle_col)
        .align_text_middle()
        .line_spacing(LINE_SPACING)
        .font_size(FONT_SIZE)
        .set(ids.middle_text, ui);
}
