use piston_window::{self, EventLoop, PistonWindow, UpdateEvent, WindowSettings};
use conrod;
use find_folder;
use conrod::backend::piston_window::GlyphCache;
use std::ops::FnMut;


// TODO make these configurable
const WIDTH: u32 = 200; // this is overridden to be screen width
const HEIGHT: u32 = 30;

pub struct Bar {
    pub window: PistonWindow,
    pub ui: conrod::Ui,
    pub text_texture_cache: GlyphCache
}


impl <'a>Bar {
    pub fn new() -> Bar {

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

        let text_texture_cache = GlyphCache::new(&mut window, WIDTH, HEIGHT);

        Bar {
            window: window,
            ui: ui,
            text_texture_cache: text_texture_cache
        }
    }


    pub fn animate_frame<F>(&mut self, set_ui: F)
        where F: Fn(conrod::UiCell,) {

        // The image map describing each of our widget->image mappings
        // (in our case, none).
        let image_map = conrod::image::Map::new();

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

        let mut text_texture_cache = GlyphCache::new(&mut window, WIDTH, HEIGHT);


        // let ref mut window = self.window;
        // let mut ui = self.ui;
        // let mut text_texture_cache = self.text_texture_cache;

        while let Some(event) = window.next() {

            // Convert the piston event to a conrod event.
            let convert = conrod::backend::piston_window::convert_event;
            if let Some(e) = convert(event.clone(), &window) {
                ui.handle_event(e);
            }

            event.update(|_| {
                set_ui(ui.set_widgets())
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
}
