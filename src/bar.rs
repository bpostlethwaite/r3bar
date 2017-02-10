use conrod::backend::piston::event::UpdateEvent;
use conrod::backend::piston::gfx::{GlyphCache, Texture, TextureSettings, Flip};
use conrod::backend::piston::window::{Size, Window, WindowSettings};
use conrod::backend::piston::{self, WindowEvents, OpenGL};
use conrod::widget::{Id, Canvas};
use conrod::{self, Widget, UiCell};
use error::BarError;
use gfx_device_gl;

// trait to enable window.size
use pistoncore_window::Window as BasicWindow;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

// TODO Height should be detected by font height
pub const DEFAULT_GAUGE_WIDTH: u32 = 200;

struct Gauge<T> {
    bind: Box<Fn(&MutexGuard<T>, Id, &mut UiCell, Option<f64>)>,
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
    pub window: Window,
    pub ui: conrod::Ui,
    image_map: conrod::image::Map<Texture<gfx_device_gl::Resources>>,
    lefts: Elems<T>,
    rights: Elems<T>,
}

impl<T: 'static> Bar<T> {
    pub fn new(height: u32) -> Bar<T> {

        // Construct the window. The starting width is overridden in the
        // patched winit library. To get the actual width we ask window.
        let win_width = 0;
        let window: Window = WindowSettings::new("RBAR", [win_width, height])
            .opengl(OpenGL::V3_2)
            .decorated(false)
            .exit_on_esc(true)
            .samples(4)
            .vsync(true)
            .build()
            .unwrap();

        let win_width = match window.size() {
            Size { width, .. } => width,
        };

        Bar {
            height: height,
            image_map: conrod::image::Map::new(),
            lefts: Vec::new(),
            rights: Vec::new(),
            ui: conrod::UiBuilder::new([win_width as f64, height as f64]).build(),
            window: window,
        }
    }

    pub fn set_fonts(&mut self, font_path: &Path) -> Result<(), BarError> {
        self.ui.fonts.insert_from_file(font_path)?;
        Ok(())
    }

    pub fn load_icons(&mut self, path: &Path) -> Result<Id, BarError> {
        let texture;
        {
            let ref mut factory = self.window.context.factory;
            let settings = TextureSettings::new();
            texture = Texture::from_path(factory, &path, Flip::None, &settings)?;
        }
        let id = self.gen_id();

        self.image_map.insert(id, texture);

        Ok(id)
    }

    pub fn animate_frame(mut self, locked: Arc<Mutex<T>>) {

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
        let image_map = self.image_map;

        // insert an unbound padding entry between rights and lefts
        elems.push(Elem::Spacer(Spacer {
            width: 0, // this will be adjusted in the rendering phase
            id: gap_id,
        }));

        // concat the right binders
        elems.extend(self.rights);

        let win_width = match window.size() {
            Size { width, .. } => width,
        };

        let mut text_texture_cache = GlyphCache::new(window, win_width, self.height);

        // Create the event loop.
        let mut events = WindowEvents::new();

        while let Some(event) = window.next_event(&mut events) {
            // Convert the piston event to a conrod event.
            let convert = piston::window::convert_event;
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
                    println!("requested gauge width {} greater than bar width {}",
                             req_width,
                             win_width)
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


                let dt = event.update_args().map(|updt| updt.dt);

                // call the bind functions on each Gauge
                for elem in elems.iter() {
                    if let &Elem::Gauge(Gauge { id, ref bind, .. }) = elem {
                        bind(&state, id, &mut ui, dt);
                    }
                }
            });

            window.draw_2d(&event, |c, g| {
                // Only re-draw if there was some change in the `Ui`.
                if let Some(primitives) = ui.draw_if_changed() {
                    fn texture_from_image<T>(img: &T) -> &T {
                        img
                    };

                    let draw = piston::gfx::draw;
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

    pub fn bind_left<F>(mut self, width: u32, bind: F) -> Bar<T>
        where F: 'static + Fn(&MutexGuard<T>, Id, &mut UiCell, Option<f64>)
    {
        let id = self.gen_id();

        self.lefts.push(Elem::Gauge(Gauge {
            bind: Box::new(bind),
            width: width,
            id: id,
        }));

        self
    }

    pub fn bind_right<F>(mut self, width: u32, bind: F) -> Bar<T>
        where F: 'static + Fn(&MutexGuard<T>, Id, &mut UiCell, Option<f64>)
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
