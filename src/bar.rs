use conrod::backend::glium::glium::{DisplayBuild, Surface};
use conrod::backend::glium::glium;
use conrod::widget::{Id, Canvas};
use conrod::{self, Widget, UiCell};
use error::BarError;
use image;
use std::path::{Path};
use std::sync::{Arc, Mutex, MutexGuard};
use std;
use std::ops::Deref;
use std::borrow::Borrow;

// 1. get ui inside thread
// 2. User Mutex instead of MutexGaurd and unlock Mutex in binder callback


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
    pub display: glium::backend::glutin_backend::GlutinFacade,
    pub height: u32,
    pub ui: conrod::Ui,
    image_map: conrod::image::Map<glium::texture::SrgbTexture2d>,
    lefts: Elems<T>,
    rights: Elems<T>,
}

impl<T: std::marker::Send + 'static> Bar<T> {
    pub fn new(height: u32) -> Bar<T> {

        // Construct the window. The starting width is overridden in the
        // patched winit library. To get the actual width we ask window.
        let win_width = 0;
        let builder = glium::glutin::WindowBuilder::new()
            .with_vsync()
            .with_dimensions(win_width, height);

        let display = builder.build_glium().unwrap();
        let mut win_width = 0;
        {
            let window = display.get_window();
            if let Some(window) = window {
                if let Some((w, _)) = window.get_inner_size() {
                    win_width = w;
                };
            }
        }

        Bar {
            height: height,
            image_map: conrod::image::Map::new(),
            lefts: Vec::new(),
            rights: Vec::new(),
            ui: conrod::UiBuilder::new([win_width as f64, height as f64]).build(),
            display: display,
        }
    }

    pub fn set_fonts(&mut self, font_path: &Path) -> Result<(), BarError> {
        self.ui.fonts.insert_from_file(font_path)?;
        Ok(())
    }

    pub fn load_image<P>(&mut self, path: P) -> conrod::image::Id
        where P: AsRef<Path>
    {
        let path = path.as_ref();
        let rgba_image = image::open(&Path::new(&path)).unwrap().to_rgba();
        let image_dimensions = rgba_image.dimensions();
        let raw_image = glium::texture::RawImage2d::from_raw_rgba_reversed(rgba_image.into_raw(),
                                                                           image_dimensions);
        let texture = glium::texture::SrgbTexture2d::new(&self.display, raw_image).unwrap();
        self.image_map.insert(texture)
    }

    pub fn animate_frame(mut self, state: Arc<Mutex<T>>) {

        // Write the requested widths into a section array. These widths
        // will be configurable but for now set to a default.
        let master_id;
        let gap_id;
        {
            let mut generator = self.ui.widget_id_generator();

            master_id = generator.next();
            gap_id = generator.next();
        }

        let ref mut display = self.display;
        let mut ui = self.ui;
        let mut elems: Elems<T> = self.lefts;
        let image_map = self.image_map;
        let mut renderer = conrod::backend::glium::Renderer::new(display).unwrap();

        // insert an unbound padding entry between rights and lefts
        elems.push(Elem::Spacer(Spacer {
            width: 0, // this will be adjusted in the rendering phase
            id: gap_id,
        }));

        // concat the right binders
        elems.extend(self.rights);

        let mut event_loop = EventLoop::new();
        'main: loop {

            println!("Starting MAIN LOOP");
            // Handle all events.
            for event in event_loop.next(&display) {
                println!("Starting EVENT LOOP");
                // Use the `winit` backend feature to convert the winit event
                // to a conrod one.
                if let Some(event) = conrod::backend::winit::convert(event.clone(), display) {
                    println!("handling event");
                    ui.handle_event(event);
                    event_loop.needs_update();
                }

                match event {
                    // Break from the loop upon `Escape`.
                    glium::glutin::Event::KeyboardInput(_, _, Some(glium::glutin::VirtualKeyCode::Escape)) |
                    glium::glutin::Event::Closed =>
                        break 'main,
                    _ => {},
                }
                println!("Finishing EVENT LOOP");
            }

            // Instantiate the widgets.
            {
                let mut win_width = 0;
                let window = display.get_window();
                if let Some(window) = window {
                    if let Some((w, _)) = window.get_inner_size() {
                        win_width = w;
                    };
                }

                // If the user has requested more than the window size we modify
                // their requested lengths until fit. <not implemented>
                let req_width = elems.width();
                if req_width > win_width {
                    println!("requested gauge width {} greater than bar width {}",
                             req_width,
                             win_width)
                }

                // Increase the spacer width to take up remaining space
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
                Canvas::new().flow_right(&splits).set(master_id, &mut ui);

                let state = state.lock().unwrap();

                // Unlock state so all binder functions may mutate.
                let dt: Option<f64> = None; //event.update_args().map(|updt| updt
                for elem in elems.iter() {
                    if let &Elem::Gauge(Gauge { id, ref bind, .. }) = elem {
                        bind(&state, id, ui, dt);
                    }
                }
            }

            // Draw the `Ui`.
            if let Some(primitives) = ui.draw_if_changed() {
                println!("DRAWING");
                renderer.fill(&display, primitives, &image_map);
                let mut target = display.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);
                renderer.draw(display, &mut target, &image_map).unwrap();
                target.finish().unwrap();
            }
            println!("finishing MAIN LOOP");
        }
    }


    fn gen_id(&mut self) -> Id {
        let mut generator = &mut self.ui.widget_id_generator();
        generator.next()
    }

    pub fn bind_left<F>(mut self, width: u32, bind: F) -> Bar<T>
        where F: 'static + std::marker::Send + Fn(&MutexGuard<T>, Id, &mut UiCell, Option<f64>)
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
        where F: 'static + std::marker::Send + Fn(&MutexGuard<T>, Id, &mut UiCell, Option<f64>)
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


pub struct EventLoop {
    ui_needs_update: bool,
    last_update: std::time::Instant,
}

impl EventLoop {

    pub fn new() -> Self {
        EventLoop {
            last_update: std::time::Instant::now(),
            ui_needs_update: true,
        }
    }

    /// Produce an iterator yielding all available events.
    pub fn next(&mut self, display: &glium::Display) -> Vec<glium::glutin::Event> {
        // We don't want to loop any faster than 60 FPS, so wait until it has been
        // at least 16ms since the last yield.
        let last_update = self.last_update;
        let sixteen_ms = std::time::Duration::from_millis(16);
        let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
        if duration_since_last_update < sixteen_ms {
            std::thread::sleep(sixteen_ms - duration_since_last_update);
        }

        // Collect all pending events.
        let mut events = Vec::new();
        events.extend(display.poll_events());

        // If there are no events and the `Ui` does not need updating,
        // wait for the next event.
        if events.is_empty() && !self.ui_needs_update {
            events.extend(display.wait_events().next());
        }

        self.ui_needs_update = false;
        self.last_update = std::time::Instant::now();

        events
    }

    // Notifies the event loop that the `Ui` requires another update whether
    // or not there are any pending events.
    //
    // This is primarily used on the occasion that some part of the `Ui` is
    // still animating and requires further updates to do so.
    pub fn needs_update(&mut self) {
        self.ui_needs_update = true;
    }
}
