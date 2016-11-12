use conrod::backend::piston_window::GlyphCache;
use conrod::{self, color, widget, Colorable, Positionable, Widget, UiCell};
use conrod::widget::{Id, Canvas};
use find_folder;
use piston_window::{self, EventLoop, PistonWindow, Size, UpdateEvent, Window, WindowSettings};
use std::sync::{Arc, Mutex, MutexGuard};

// TODO Height should be detected by font height
const WIDTH: u32 = 200; // this is overridden to be screen width
const HEIGHT: u32 = 30;

const MAX_GAUGES: u32 = 9;
const GAUGE_WIDTH: u32 = 200;

struct Binder<T> {
    binder: Box<Fn(&MutexGuard<T>, Id, &mut UiCell)>,
    width: u32,
    id: Id,
}

pub struct Bar<T> {
    pub window: PistonWindow,
    pub ui: conrod::Ui,
    lefts: Vec<Binder<T>>,
    rights: Vec<Binder<T>>,
}

// [{binder: binder, width: width}, {binder: binder, width: width}]


impl<T: 'static> Bar<T> {
    pub fn new() -> Bar<T> {

        // Construct the window.
        let mut window: PistonWindow = WindowSettings::new("RBAR", [WIDTH, HEIGHT])
            .opengl(piston_window::OpenGL::V3_2)
            .decorated(false)
            .exit_on_esc(true)
            .samples(4)
            .vsync(true)
            .build()
            .unwrap();
        window.set_ups(60);

        // Construct our `Ui`.
        let mut ui = conrod::UiBuilder::new().build();

        // Add a `Font` to the `Ui`'s `font::Map` from file.
        let assets = find_folder::Search::KidsThenParents(3, 5).for_folder("assets").unwrap();
        let font_path = assets.join("fonts/Roboto Mono for Powerline.ttf");
        ui.fonts.insert_from_file(font_path).unwrap();

        Bar {
            window: window,
            ui: ui,
            lefts: Vec::new(),
            rights: Vec::new(),
        }
    }

    pub fn animate_frame(mut self, locked: Arc<Mutex<T>>) {

        // The image map describing each of our widget->image mappings
        // (in our case, none).
        let image_map = conrod::image::Map::new();

        // The maximum number of gauges bar supports
        let mut sections: [u32; MAX_GAUGES as usize] = [0; MAX_GAUGES as usize];

        let ref mut window = self.window;
        let mut ui = self.ui;
        let lefts = self.lefts;
        let rights = self.rights;

        // let segments = self.lefts;
        // segments.extend(rights);

        // Write the requested widths into a section array. These widths
        // will be configurable but for now set to a default.
        let n_gauges = (lefts.len() + rights.len()) as u32;

        let master_id;
        let left_id;
        let right_id;
        {
            let mut generator = ui.widget_id_generator();

            master_id = generator.next();
            left_id = generator.next();
            right_id = generator.next();
        }

        let mut text_texture_cache = GlyphCache::new(window, WIDTH, HEIGHT);

        while let Some(event) = window.next() {

            // Convert the piston event to a conrod event.
            let convert = conrod::backend::piston_window::convert_event;
            if let Some(e) = convert(event.clone(), &window) {
                ui.handle_event(e);
            }

            event.update(|_| {


                // Set up a series of rectangles that respect requested
                // user widths and screen dimensions.
                let width = match window.size() {
                    Size {width, .. } => width
                };

                // We only care about sections that the user is going to bind
                // so tally those (they will eventually support unique sizing
                // but for now it is just the default).
                let req_width = GAUGE_WIDTH * n_gauges;

                // If the user has requested more than the window size we modify
                // their requested lengths until fit. <not implemented>
                if req_width > width {
                    panic!("requested gauge widths greater than bar width")
                }

                let mut ui = &mut ui.set_widgets();

                let left_canvas = Canvas::new().color(color::DARK_CHARCOAL);
                let right_canvas = Canvas::new().color(color::DARK_CHARCOAL);

                let mut left_splits = Vec::new();
                for &Binder {id, ..} in lefts.iter() {
                    left_splits.push((id, Canvas::new().color(color::DARK_CHARCOAL)));
                }

                let mut right_splits = Vec::new();
                for &Binder {id, ..} in rights.iter() {
                    right_splits.push((id, Canvas::new().color(color::DARK_CHARCOAL)));
                }

                let left_canvas = left_canvas.flow_right(&left_splits);
                let right_canvas = right_canvas.flow_left(&right_splits);

                Canvas::new()
                    .flow_right(&[(left_id, left_canvas), (right_id, right_canvas)])
                    .set(master_id, &mut ui);


                // Unlock state so all binder functions may mutate.
                let state = locked.lock().unwrap(); // TODO

                for &Binder {id, ref binder, .. } in lefts.iter() {
                    binder(&state, id, &mut ui);
                }

                for &Binder {id, ref binder, .. } in rights.iter() {
                    binder(&state, id, &mut ui);
                }
            });

            window.draw_2d(&event, |c, g| {
                // Only re-draw if there was some change in the `Ui`.
                if let Some(primitives) = ui.draw_if_changed() {
                    fn texture_from_image<T>(img: &T) -> &T {
                        img
                    };

                    let draw = conrod::backend::piston_window::draw;
                    draw(c,
                         g,
                         primitives,
                         &mut text_texture_cache,
                         &image_map,
                         texture_from_image);
                }
            });
        }
    }

    fn gen_id(&mut self) -> Id {

        let mut generator = &mut self.ui.widget_id_generator();

        generator.next()

    }

    pub fn bind_left<F>(mut self, binder: F) -> Bar<T>
        where F: 'static + Fn(&MutexGuard<T>, Id, &mut UiCell)
    {

        let id = self.gen_id();

        self.lefts.push(Binder{
            binder: Box::new(binder),
            width: GAUGE_WIDTH,
            id: id,
        });

        self
    }

    pub fn bind_right<F>(mut self, binder: F) -> Bar<T>
        where F: 'static + Fn(&MutexGuard<T>, Id, &mut UiCell)
    {
        let id = self.gen_id();

        self.rights.insert(0, Binder{
            binder: Box::new(binder),
            width: GAUGE_WIDTH,
            id: id,
        });

        return self;
    }
}
