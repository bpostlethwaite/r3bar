use conrod::backend::glium::glium;
use conrod::widget::{Id, Canvas};
use conrod::{self, Widget, UiCell};
use display::display;
use error::BarError;
use image;
use self::glium::glutin::Event::KeyboardInput;
use self::glium::glutin::VirtualKeyCode as KeyCode;
use self::glium::{DisplayBuild, Surface};
use std::marker::{Send, Sync};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc};
use std::time::Duration;
use std;

// 1. get ui inside thread
// 2. User Mutex instead of MutexGaurd and unlock Mutex in binder callback
// https://gist.github.com/anonymous/d1b3dfbabe5dac2995da37da41d18625

// TODO Height should be detected by font height
pub const DEFAULT_GAUGE_WIDTH: u32 = 200;

struct Gauge {
    bind: Box<Fn(Id, &mut UiCell, &mut UpdateConfig)>,
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

    pub fn run<F, T>(&self,
                  height: u32,
                  app_tx: mpsc::Sender<T>,
                  ui_renderer: Arc<F>) -> Vec<mpsc::Sender<DispResponse>>
        where F: Fn(&mut UiLoop, mpsc::Sender<T>) + 'static + Sync + Send,
              T: 'static + Sync + Send
    {

        let mut ui_txs = Vec::new();

        let outputs = vec!["eDP1", "HDMI2"];
        for output in outputs {
            let app_tx = app_tx.clone();

            // A channel to send events from the display thread to the conrod thread.
            let (ui_tx, ui_rx) = mpsc::channel();
            ui_txs.push(ui_tx.clone());

            // A channel to send `render::Primitive`s from the conrod thread to
            // the `winit thread.
            let (disp_tx, disp_rx) = mpsc::channel();

            std::thread::spawn(move || {
                DisplayLoop::run(
                    height, output.to_owned(), ui_tx, disp_rx
                );
            });
            let renderer = ui_renderer.clone();
            std::thread::spawn(move || {
                UiLoop::run(
                    renderer, output.to_owned(), ui_rx, disp_tx, app_tx
                );
            });
        }

        ui_txs
    }
}

pub struct DisplayInfo {
    proxy: glium::glutin::WindowProxy,
    width: u32,
    height: u32,
}

pub enum UiRequest {
    DisplayInfo,
    ImageId(PathBuf),
    Primitives(conrod::render::OwnedPrimitives),
}

pub enum DispResponse {
    DisplayInfo(DisplayInfo),
    Event(conrod::event::Input),
    ImageId(conrod::image::Id),
    WakeDisplay,
}


struct DisplayLoop {
    display: display::R3Display,
    image_map: conrod::image::Map<glium::texture::SrgbTexture2d>,
}

impl DisplayLoop {
    fn run(height: u32,
           output: String,
           tx: mpsc::Sender<DispResponse>,
           rx: mpsc::Receiver<UiRequest>) {

        // Construct the window. The starting width is overridden in the
        // patched winit library. To get the actual width we ask window.

        let builder = match output.as_ref() {
            "eDP1" => {
                glium::glutin::WindowBuilder::new()
                    .with_vsync()
                    .with_decorations(true)
                    .with_dimensions(1920, height)
            },
            "HDMI2" => {
                glium::glutin::WindowBuilder::new()
                    .with_vsync()
                    .with_decorations(false)
                    .with_dimensions(2560, height)
            },
            _ => return
        };

        let window = builder.build().unwrap();
        let display = display::R3Display::new(Rc::new(window));
        let dloop = &mut DisplayLoop{
            display: display,
            image_map: conrod::image::Map::new(),
        };

        dloop.process_events(tx, rx);
    }

    pub fn load_image<P>(&mut self, path: P) -> conrod::image::Id
        where P: AsRef<Path>
    {
        let path = path.as_ref();
        let rgba_image = image::open(&Path::new(&path)).unwrap().to_rgba();
        let image_dimensions = rgba_image.dimensions();
        let raw_image = glium::texture::RawImage2d::from_raw_rgba_reversed(
            rgba_image.into_raw(), image_dimensions
        );
        let texture = glium::texture::SrgbTexture2d::new(
            &self.display, raw_image
        ).unwrap();
        self.image_map.insert(texture)
    }

    fn display_info(&self) -> DisplayInfo {
        let (width, height) = self.window_dims();
        let proxy = self.display.get_window().create_window_proxy();

        DisplayInfo{
            proxy: proxy,
            width: width,
            height: height,
        }
    }

    fn process_events(&mut self,
                      tx: mpsc::Sender<DispResponse>,
                      rx: mpsc::Receiver<UiRequest>) {

        let mut last_update = std::time::Instant::now();
        let mut renderer = conrod::backend::glium::Renderer::new(
            &self.display
        ).unwrap();

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
                    event.clone(), self.display.get_window()
                ) {
                    tx.send(DispResponse::Event(event)).unwrap();
                }

                match event {
                    // Break from the loop upon `Escape`.
                    KeyboardInput(_, _, Some(KeyCode::Escape)) |
                    glium::glutin::Event::Closed =>
                        break 'main,
                    _ => {},
                }
            }

            // Process msgs until all msgs have been consumed and we have
            // obtained at least one primitive to render.
            // Only draw the last primitive from the queue (ignore the others).
            let mut maybe_primitives = None;
            while let Ok(resp) = rx.try_recv() {
                match resp {
                    UiRequest::Primitives(next_primitives) => {
                        maybe_primitives = Some(next_primitives);
                    },

                    UiRequest::DisplayInfo => {
                        let info_resp = self.display_info();
                        tx.send(DispResponse::DisplayInfo(info_resp)).unwrap();
                    },

                    UiRequest::ImageId(path) => {
                        let id = self.load_image(path);
                        tx.send(DispResponse::ImageId(id)).unwrap();
                    },
                }
            }

            if let Some(primitives) = maybe_primitives {
                renderer.fill(&self.display, primitives.walk(), &self.image_map);

                let mut target = self.display.draw();
                target.clear_color(0.0, 0.0, 0.0, 1.0);

                renderer.draw(&self.display, &mut target, &self.image_map).unwrap();

                target.finish().unwrap();
            }

            last_update = std::time::Instant::now();
        }
    }


    fn window_dims(&self) -> (u32, u32) {
        let window = self.display.get_window();
        if let Some(window) = window {
            if let Some(dims) = window.get_inner_size() {
                return dims
            };
        }
        (0, 0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UpdateConfig {
    needs_update: bool,
    last_update: std::time::Instant,
}

impl UpdateConfig {
    pub fn since_last_update(&self) -> Duration {
        let now = std::time::Instant::now();
        now.duration_since(self.last_update)
    }

    pub fn update(&mut self) {
        self.needs_update = true;
    }

    fn updated(&mut self) {
        self.last_update = std::time::Instant::now();
    }
}



pub struct UiLoop {
    pub ui: conrod::Ui,
    pub display_info: DisplayInfo,
    pub output: String,
    lefts: Elems,
    rights: Elems,
    rx: mpsc::Receiver<DispResponse>,
    tx: mpsc::Sender<UiRequest>
}

impl UiLoop {
    fn run<F, T>(ui_renderer: Arc<F>,
                 output: String,
                 rx: mpsc::Receiver<DispResponse>,
                 tx: mpsc::Sender<UiRequest>,
                 maybe_app_tx: mpsc::Sender<T>,
    )
        where F: 'static + Sync + Send + Fn(&mut UiLoop, mpsc::Sender<T>),
              T: Sync + Send
    {


        // Send request for display info.
        tx.send(UiRequest::DisplayInfo).unwrap();

        // continue to listen until we receive it
        while let Ok(resp) = rx.recv() {
            match resp {
                DispResponse::DisplayInfo(info) => {
                    let dims = [info.width as f64, info.height as f64];
                    let ui_loop = UiLoop{
                        ui: conrod::UiBuilder::new(dims).build(),
                        display_info: info,
                        lefts: Vec::new(),
                        rights: Vec::new(),
                        output: output,
                        rx: rx,
                        tx: tx,
                    };
                    ui_loop.process_ui(ui_renderer, maybe_app_tx);
                    break;
                }
                _ => continue,
            }
        }
    }

    pub fn set_fonts(&mut self, font_path: &Path) -> Result<(), BarError> {
        self.ui.fonts.insert_from_file(font_path)?;
        Ok(())
    }

    fn gen_id(&mut self) -> Id {
        self.ui.widget_id_generator().next()
    }

    pub fn load_image(&self, p: PathBuf) -> Result<conrod::image::Id, BarError> {

        self.tx.send(UiRequest::ImageId(p)).unwrap();

        // wake up display thread in case it is blocking
        self.display_info.proxy.wakeup_event_loop();

        // continue to listen until we receive it
        while let Ok(resp) = self.rx.recv() {
            match resp {
                DispResponse::ImageId(id) => {
                    return Ok(id);
                }
                _ => continue,
            }
        }

        return Err(BarError::Bar(format!("{}", "Some damn image id error")));
    }

    pub fn bind_left<F>(&mut self, width: u32, bind: F) -> &Self
        where F: 'static + Send + Fn(Id, &mut UiCell, &mut UpdateConfig)
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
        where F: 'static + Send + Fn(Id, &mut UiCell, &mut UpdateConfig)
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

    fn process_ui<F, T>(mut self,
                     ui_renderer: Arc<F>,
                     app_tx: mpsc::Sender<T>)
        where F: 'static + Sync + Send + Fn(&mut UiLoop, mpsc::Sender<T>),
              T: Sync + Send
    {

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
            ui_renderer(&mut self, app_tx);
        }

        let mut elems: Elems = self.lefts;

        // insert an unbound padding entry between rights and lefts
        elems.push(Elem::Spacer(Spacer {
            width: 0, // this will be adjusted in the rendering phase
            id: gap_id,
        }));

        // concat the right binders
        elems.extend(self.rights);

        let mut updater = UpdateConfig{
            needs_update: true,
            last_update: std::time::Instant::now(),
        };

        'conrod: loop {

            // Collect any pending events.
            let mut events = Vec::new();
            while let Ok(event) = self.rx.try_recv() {
                match event {
                    DispResponse::DisplayInfo(info) => self.display_info = info,
                    DispResponse::ImageId(_) => (),
                    DispResponse::Event(event) => events.push(event),
                    DispResponse::WakeDisplay => {
                        self.display_info.proxy.wakeup_event_loop();
                    },
                }
            }

            // If there are no events pending, wait for them.
            if events.is_empty() || !updater.needs_update {
                match self.rx.recv() {
                    Ok(DispResponse::DisplayInfo(info)) => self.display_info = info,
                    Ok(DispResponse::ImageId(_)) => (),
                    Ok(DispResponse::Event(event)) => events.push(event),
                    Ok(DispResponse::WakeDisplay) => {
                        self.display_info.proxy.wakeup_event_loop();
                    },
                    Err(_) => break 'conrod,
                }
            }

            updater.needs_update = false;

            // Input each event into the `Ui`.
            for event in events {
                self.ui.handle_event(event);
                updater.needs_update = true;
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
                for elem in elems.iter() {
                    if let &Elem::Gauge(Gauge { id, ref bind, .. }) = elem {
                        bind(id, ui, &mut updater);
                    }
                }
            }

            // Render the `Ui` to a list of primitives that we can send to the
            // main thread for display.
            if let Some(primitives) = self.ui.draw_if_changed() {
                if self.tx.send(UiRequest::Primitives(primitives.owned())).is_err() {
                    break 'conrod;
                }
                // Wakeup `winit` for rendering.
                self.display_info.proxy.wakeup_event_loop();

            }

            updater.updated();
        }

    }
}
