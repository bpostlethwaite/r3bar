use conrod::backend::piston_window::GlyphCache;
use conrod::{color, widget, Colorable, Widget};
use conrod;
use find_folder;
use piston_window::{self, EventLoop, PistonWindow, UpdateEvent, WindowSettings};
use std::sync::{Arc, Mutex, MutexGuard};

// TODO Height should be detected by font height
const WIDTH: u32 = 200; // this is overridden to be screen width
const HEIGHT: u32 = 30;

pub struct Bar<T> {
    pub window: PistonWindow,
    pub ui: conrod::Ui,
    binders: Vec<Box<Fn(&MutexGuard<T>, conrod::widget::Id, &mut conrod::UiCell,)>>,
    ids: Vec<conrod::widget::Id>,
}


impl <T: 'static>Bar<T> {
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

        let master_id;
        {
            let mut generator = ui.widget_id_generator();
            master_id = generator.next();
        }
        let mut ids: Vec<conrod::widget::Id> = Vec::new();
        ids.push(master_id);

        Bar {
            window: window,
            ui: ui,
            binders: Vec::new(),
            ids: ids,
        }
    }

    pub fn animate_frame(mut self, locked: Arc<Mutex<T>>) {

        // The image map describing each of our widget->image mappings
        // (in our case, none).
        let image_map = conrod::image::Map::new();

        let ref mut window = self.window;
        let mut ui = self.ui;
        let binders = &self.binders;

        let mut text_texture_cache = GlyphCache::new(window, WIDTH, HEIGHT);

        let ids = self.ids;

        while let Some(event) = window.next() {

            // Convert the piston event to a conrod event.
            let convert = conrod::backend::piston_window::convert_event;
            if let Some(e) = convert(event.clone(), &window) {
                ui.handle_event(e);
            }

            event.update(|_| {
                let mut ui = &mut ui.set_widgets();
                let mut splits = Vec::new();

                if let Some((&master_id, widget_ids)) = ids.split_first() {
                    for &id in widget_ids {
                        splits.push(
                            (id, widget::Canvas::new().color(color::DARK_CHARCOAL)));
                    }
                    widget::Canvas::new().flow_right(&splits)
                        .set(master_id, &mut ui);

                    let state = locked.lock().unwrap(); // TODO
                    for (&widget_id, binder) in widget_ids.iter().zip(binders) {
                        binder(&state, widget_id, &mut ui);
                    }
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

    pub fn bind_widget<F>(mut self, binder: F) -> Bar<T>
        where F: 'static + Fn(&MutexGuard<T>, conrod::widget::Id, &mut conrod::UiCell,) {
        self.binders.push(Box::new(binder));

        // add a new id for each new primary widget
        let id = self.ui.widget_id_generator().next();
        self.ids.push(id);

        return self;
    }
}
