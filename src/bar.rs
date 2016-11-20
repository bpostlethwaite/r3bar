use conrod::backend::piston_window::GlyphCache;
use conrod::widget::{Id, Canvas};
use conrod::{self, Widget, UiCell};
use piston_window::{self, EventLoop, PistonWindow, Size, UpdateEvent, Window, WindowSettings};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use error::BarError;

// TODO Height should be detected by font height
pub type Width = u32;
pub const DEFAULT_GAUGE_WIDTH: Width = 200;

const WIDTH: u32 = 200; // this is overridden to be screen width


struct Gauge<T> {
    bind: Box<Fn(&MutexGuard<T>, Id, &mut UiCell)>,
    width: u32,
    id: Id,
}

struct Spacer {
    width: u32,
    id: Id,
}

enum Elem<T> {
    Spacer(Spacer),
    Gauge(Gauge<T>),
}

type Elems<T> = Vec<Elem<T>>;

trait ElemList {
    fn width(&self) -> u32;
}

impl<T> ElemList for Elems<T> {
    fn width(&self) -> u32 {
        let mut total_width = 0;

        for elem in self.iter() {
            let width = match elem {
                &Elem::Gauge(Gauge { width, .. }) => width,
                &Elem::Spacer(Spacer { width, .. }) => width,
            };

            total_width = total_width + width;
        }
        total_width
    }
}


pub struct Bar<T> {
    pub height: u32,
    pub window: PistonWindow,
    pub ui: conrod::Ui,
    lefts: Elems<T>,
    rights: Elems<T>,
}

// NOTE INSTEAD OF OPTION binder field use an ENUM in the lefts and rights
// vecs. could be type GAUGE, SPACER and maybe SEPERATOR


impl<T: 'static> Bar<T> {
    pub fn new(height: u32) -> Bar<T> {

        // Construct the window.
        let mut window: PistonWindow = WindowSettings::new("RBAR", [WIDTH, height])
            .opengl(piston_window::OpenGL::V3_2)
            .decorated(false)
            .exit_on_esc(true)
            .samples(4)
            .vsync(true)
            .build()
            .unwrap();
        window.set_ups(60);

        Bar {
            height: height,
            lefts: Vec::new(),
            rights: Vec::new(),
            ui: conrod::UiBuilder::new().build(),
            window: window,
        }
    }

    pub fn set_fonts(&mut self, font_path: &Path) -> Result<(), BarError> {
        self.ui.fonts.insert_from_file(font_path)?;
        Ok(())
    }

    pub fn animate_frame(mut self, locked: Arc<Mutex<T>>) {

        // The image map describing each of our widget->image mappings
        // (in our case, none).
        let image_map = conrod::image::Map::new();

        // Write the requested widths into a section array. These widths
        // will be configurable but for now set to a default.
        let master_id;
        let gap_id;
        {
            let mut generator = self.ui.widget_id_generator();

            master_id = generator.next();
            gap_id = generator.next();
        }

        let ref mut window = self.window;
        let mut ui = self.ui;
        let mut elems: Elems<T> = self.lefts;

        // insert an unbound padding entry between rights and lefts
        elems.push(Elem::Spacer(Spacer {
            width: 0, // this will be adjusted in the rendering phase
            id: gap_id,
        }));

        // concat the right binders
        elems.extend(self.rights);

        let mut text_texture_cache = GlyphCache::new(window, WIDTH, self.height);

        while let Some(event) = window.next() {
            // Convert the piston event to a conrod event.
            let convert = conrod::backend::piston_window::convert_event;
            if let Some(e) = convert(event.clone(), &window) {
                ui.handle_event(e);
            }

            event.update(|_| {


                // Set up a series of rectangles that respect requested
                // user widths and screen dimensions.
                let win_width = match window.size() {
                    Size { width, .. } => width,
                };

                // If the user has requested more than the window size we modify
                // their requested lengths until fit. <not implemented>
                let req_width = elems.width();
                if req_width > win_width {
                    panic!("requested gauge widths greater than bar width")
                }

                // Next increase the spacer width to take up remaining space
                // if we have additional room to fill.
                let rem_width = win_width - req_width;
                if rem_width > 0 {
                    for elem in &mut elems {
                        if let &mut Elem::Spacer(ref mut spacer) = elem {
                            spacer.width = rem_width;
                            break;
                        }
                    }
                }

                let mut splits = Vec::with_capacity(elems.len());
                for elem in elems.iter() {
                    let (width, id) = match elem {
                        &Elem::Gauge(Gauge { width, id, .. }) => (width, id),
                        &Elem::Spacer(Spacer { width, id }) => (width, id),
                    };

                    splits.push((id, Canvas::new().length(width as f64)));
                }

                let mut ui = &mut ui.set_widgets();

                // main background canvas
                Canvas::new().flow_right(&splits).set(master_id, &mut ui);

                // Unlock state so all binder functions may mutate.
                let state = locked.lock().unwrap(); // TODO

                // call the bind functions on each Gauge
                for elem in elems.iter() {
                    if let &Elem::Gauge(Gauge { id, ref bind, .. }) = elem {
                        bind(&state, id, &mut ui);
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

    fn gen_id(&mut self) -> Id {
        let mut generator = &mut self.ui.widget_id_generator();
        generator.next()
    }

    pub fn bind_left<F>(mut self, width: Width, bind: F) -> Bar<T>
        where F: 'static + Fn(&MutexGuard<T>, Id, &mut UiCell)
    {
        let id = self.gen_id();

        self.lefts.push(Elem::Gauge(Gauge {
            bind: Box::new(bind),
            width: width,
            id: id,
        }));

        self
    }

    pub fn bind_right<F>(mut self, width: Width, bind: F) -> Bar<T>
        where F: 'static + Fn(&MutexGuard<T>, Id, &mut UiCell)
    {
        let id = self.gen_id();

        self.rights.insert(0,
                           Elem::Gauge(Gauge {
                               bind: Box::new(bind),
                               width: width,
                               id: id,
                           }));

        self
    }
}
