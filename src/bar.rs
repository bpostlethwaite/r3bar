use conrod::backend::glium::glium::{DisplayBuild, Surface};
use conrod::backend::glium::glium;
use conrod::widget::{Id, Canvas};
use conrod::{self, Widget, UiCell};
use error::BarError;
use image;
use std::path::{Path};
use std::sync::{Arc};
use std;

// 1. get ui inside thread
// 2. User Mutex instead of MutexGaurd and unlock Mutex in binder callback
// https://gist.github.com/anonymous/d1b3dfbabe5dac2995da37da41d18625

// TODO Height should be detected by font height
pub const DEFAULT_GAUGE_WIDTH: u32 = 200;

struct Gauge {
    bind: Box<Fn(Id, &mut UiCell, Option<f64>)>,
    width: u32,
    id: Id,
}

struct Spacer {
    width: u32,
    id: Id,
}

enum Elem {
    Spacer(Spacer),
    Gauge(Gauge),
}

type Elems = Vec<Elem>;

trait ElemList {
    fn width(&self) -> u32;
}

impl ElemList for Elems {
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

pub struct Bar {}

impl Bar {

    pub fn run(&self, height: u32, ui_renderer: Arc<Fn(&mut UiLoop) + std::marker::Sync +  std::marker::Send + 'static>) {

        let monitors = glium::glutin::get_available_monitors();
        for monitor_id in monitors {

            // A channel to send events from the main `winit` thread to the
            // conrod thread.
            let (event_tx, event_rx) = std::sync::mpsc::channel();

            // A rendezvous (blocking) channel to send display info & proxy from
            // the main `winit` thread to the conrod thread. The window proxy will
            // allow conrod to wake up the `winit::Window` for rendering.
            let (display_tx, display_rx) = std::sync::mpsc::sync_channel(0);

            // A channel to send `render::Primitive`s from the conrod thread to
            // the `winit thread.
            let (render_tx, render_rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                DisplayLoop::run(
                    height, monitor_id, display_tx, event_tx, render_rx
                );
            });

            let renderer = ui_renderer.clone();
            std::thread::spawn(move || {
                UiLoop::run(
                    renderer, display_rx, event_rx, render_tx
                );
            });
        }
    }
}

struct DisplayInfo {
    proxy: glium::glutin::WindowProxy,
    width: u32,
    height: u32,
}

enum Request {
    DisplayInfo,
    ImageId,
    Primitives(conrod::render::OwnedPrimitives),
}

enum Response {
    DisplayInfo(DisplayInfo),
    Event(conrod::event::Input),
    ImageId(conrod::image::Id),
}


struct DisplayLoop {
    display: glium::backend::glutin_backend::GlutinFacade,
    image_map: conrod::image::Map<glium::texture::SrgbTexture2d>,
}

impl DisplayLoop {
    fn run(height: u32,
           monitor_id: glium::glutin::MonitorId,
           display_tx: std::sync::mpsc::SyncSender<DisplayInfo>,
           event_tx: std::sync::mpsc::Sender<conrod::event::Input>,
           render_rx: std::sync::mpsc::Receiver<conrod::render::OwnedPrimitives>) {

        // Construct the window. The starting width is overridden in the
        // patched winit library. To get the actual width we ask window.
        let win_width = 0;
        let builder = glium::glutin::WindowBuilder::new()
            .with_vsync()
            .with_dimensions(win_width, height);

        let display = builder.build_glium().unwrap();
        let dloop = DisplayLoop{
            display: display,
            image_map: conrod::image::Map::new(),
        };

        let window_width = dloop.window_width();
        let proxy = dloop.display.get_window().unwrap().create_window_proxy();

        let info = DisplayInfo{
            proxy: proxy,
            width: window_width,
            height: height,
        };

        display_tx.send(info).unwrap();

        dloop.process_events(event_tx, render_rx);
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

    fn process_events(&self,
                      event_tx: std::sync::mpsc::Sender<conrod::event::Input>,
                      render_rx: std::sync::mpsc::Receiver<conrod::render::OwnedPrimitives>) {
        let mut last_update = std::time::Instant::now();
        let mut renderer = conrod::backend::glium::Renderer::new(&self.display).unwrap();
        'main: loop {

            // We don't want to loop any faster than 60 FPS, so wait until it has been at least
            // 16ms since the last yield.
            let sixteen_ms = std::time::Duration::from_millis(16);
            let now = std::time::Instant::now();
            let duration_since_last_update = now.duration_since(last_update);
            if duration_since_last_update < sixteen_ms {
                std::thread::sleep(sixteen_ms - duration_since_last_update);
            }

            // Collect all pending events.
            let mut events: Vec<_> = self.display.poll_events().collect();

            // If there are no events, wait for the next event.
            if events.is_empty() {
                events.extend(self.display.wait_events().next());
            }

            // Send any relevant events to the conrod thread.
            for event in events {

                // Use the `winit` backend feature to convert the winit event
                // to a conrod one.
                if let Some(event) = conrod::backend::winit::convert(
                    event.clone(), &self.display
                ) {
                    event_tx.send(event).unwrap();
                }

                match event {
                    // Break from the loop upon `Escape`.
                    glium::glutin::Event::KeyboardInput(_, _, Some(glium::glutin::VirtualKeyCode::Escape)) |
                    glium::glutin::Event::Closed =>
                        break 'main,
                    _ => {},
                }
            }

            // Draw the most recently received `conrod::render::Primitives`
            // sent from the `Ui`.
            if let Ok(mut primitives) = render_rx.try_recv() {
                while let Ok(newest) = render_rx.try_recv() {
                    primitives = newest;
                }

                renderer.fill(&self.display, primitives.walk(), &self.image_map);
                let mut target = self.display.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);
                renderer.draw(&self.display, &mut target, &self.image_map).unwrap();
                target.finish().unwrap();
            }

            last_update = std::time::Instant::now();
        }
    }


    fn window_width(&self) -> u32 {
        let mut win_width = 0;
        {
            let window = self.display.get_window();
            if let Some(window) = window {
                if let Some((w, _)) = window.get_inner_size() {
                    win_width = w;
                };
            }
        }
        win_width
    }
}

pub struct UiLoop {
    pub ui: conrod::Ui,
    display_info: DisplayInfo,
    lefts: Elems,
    rights: Elems,
}

impl UiLoop {
    fn run(ui_renderer: Arc<Fn(&mut UiLoop) + std::marker::Sync + std::marker::Send + 'static>,
           display_rx: std::sync::mpsc::Receiver<DisplayInfo>,
           event_rx: std::sync::mpsc::Receiver<conrod::event::Input>,
           render_tx: std::sync::mpsc::Sender<conrod::render::OwnedPrimitives>) {

        let d = display_rx.recv().unwrap();

        let ui_loop = UiLoop{
            ui: conrod::UiBuilder::new([d.width as f64, d.height as f64]).build(),
            display_info: d,
            lefts: Vec::new(),
            rights: Vec::new(),
        };

        ui_loop.process_ui(ui_renderer, event_rx, render_tx);
    }

    pub fn set_fonts(&mut self, font_path: &Path) -> Result<(), BarError> {
        self.ui.fonts.insert_from_file(font_path)?;
        Ok(())
    }

    fn gen_id(&mut self) -> Id {
        let mut generator = &mut self.ui.widget_id_generator();
        generator.next()
    }

    pub fn bind_left<F>(&mut self, width: u32, bind: F) -> &Self
        where F: 'static + std::marker::Send + Fn(Id, &mut UiCell, Option<f64>)
    {
        let id = self.gen_id();

        self.lefts.push(Elem::Gauge(Gauge {
            bind: Box::new(bind),
            width: width,
            id: id,
        }));

        self
    }

    pub fn bind_right<F>(&mut self, width: u32, bind: F) -> &Self
        where F: 'static + std::marker::Send + Fn(Id, &mut UiCell, Option<f64>)
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

    fn process_ui(mut self,
                  ui_renderer: Arc<Fn(&mut UiLoop) + std::marker::Sync + 'static>,
                  event_rx: std::sync::mpsc::Receiver<conrod::event::Input>,
                  render_tx: std::sync::mpsc::Sender<conrod::render::OwnedPrimitives>) {

        // Write the requested widths into a section array. These widths
        // will be configurable but for now set to a default.
        let master_id;
        let gap_id;
        {
            let mut generator = self.ui.widget_id_generator();

            master_id = generator.next();
            gap_id = generator.next();
        }

        {
            ui_renderer(&mut self);
        }

        let mut elems: Elems = self.lefts;

        // insert an unbound padding entry between rights and lefts
        elems.push(Elem::Spacer(Spacer {
            width: 0, // this will be adjusted in the rendering phase
            id: gap_id,
        }));

        // concat the right binders
        elems.extend(self.rights);


        let mut needs_update = true;
        'conrod: loop {

            // Collect any pending events.
            let mut events = Vec::new();
            while let Ok(event) = event_rx.try_recv() {
                events.push(event);
            }

            // If there are no events pending, wait for them.
            if events.is_empty() || !needs_update {
                match event_rx.recv() {
                    Ok(event) => events.push(event),
                    Err(_) => break 'conrod,
                };
            }

            needs_update = false;

            // Input each event into the `Ui`.
            for event in events {
                self.ui.handle_event(event);
                needs_update = true;
            }

            // If the user has requested more than the window size we modify
            // their requested lengths until fit. <not implemented>
            let req_width = elems.width();
            if req_width > self.display_info.width {
                println!("requested gauge width {} greater than bar width {}",
                         req_width,
                         self.display_info.width)
            }

            // Increase the spacer width to take up remaining space
            // if we have additional room to fill.
            let rem_width = self.display_info.width- req_width;
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

            {
                let mut ui = &mut self.ui.set_widgets();
                Canvas::new().flow_right(&splits).set(master_id, ui);

                // Unlock state so all binder functions may mutate.
                let dt: Option<f64> = None; //event.update_args().map(|updt| updt
                for elem in elems.iter() {
                    if let &Elem::Gauge(Gauge { id, ref bind, .. }) = elem {
                        bind(id, ui, dt);
                    }
                }
            }

            // Render the `Ui` to a list of primitives that we can send to the
            // main thread for display.
            if let Some(primitives) = self.ui.draw_if_changed() {
                if render_tx.send(primitives.owned()).is_err() {
                    break 'conrod;
                }
                // Wakeup `winit` for rendering.
                self.display_info.proxy.wakeup_event_loop();
            }
        }

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
