use conrod::backend::glium::glium;
use conrod::widget::{Id, Canvas};
use conrod::{self, Positionable, Sizeable, Widget, UiCell};
use conrod::event::Drag;
use error::BarError;
use image;
use self::glium::glutin::Event::KeyboardInput;
use self::glium::glutin::VirtualKeyCode as KeyCode;
use self::glium::{DisplayBuild, Surface};
use std::marker::{Send, Sync};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc};
use std::time::Duration;
use std;
use widgets::sep::Sep;

// 1. get ui inside thread
// 2. User Mutex instead of MutexGaurd and unlock Mutex in binder callback
// https://gist.github.com/anonymous/d1b3dfbabe5dac2995da37da41d18625

// TODO Height should be detected by font height
pub const DEFAULT_GAUGE_WIDTH: u32 = 200;
pub const DEFAULT_SEP_WIDTH: u32 = 24;

struct Binder {
    bind: Box<Fn(Id, &mut UiCell, UpdateConfig) -> UpdateConfig> ,
    id: Id,
    width: u32,
    layout: super::Layout,
    update: UpdateConfig,
}

trait BinderList {
    fn update_widths(&mut self, &conrod::Ui);
    fn total_width(&self) -> u32;
}

impl BinderList for Vec<Binder> {

    fn total_width(&self) -> u32 {
        self.iter().fold(0, |sum, c| sum + c.width)
    }

    fn update_widths(&mut self, ui: &conrod::Ui) {
        for b in self.iter_mut() {
            let &mut Binder{width: prevw, layout, id, ..} = b;

            let mut w = match layout.width {
                Some(w) => {
                    println!("has width: {}", w);
                    w
                },
                None => {

                    // if a width isn't set get the width from node contents
                    let bbox = ui.kids_bounding_box(id);

                    // TODO also consider effects of padding & margins?
                    match bbox.map(|rect| rect.x.len()) {
                        Some(w) => w as u32,
                        None => DEFAULT_GAUGE_WIDTH,
                    }
                }
            };

            if let Some(min_w) = layout.minwidth {
                if w < min_w {
                    w = min_w;
                }
            }

            if let Some(max_w) = layout.maxwidth {
                if w > max_w {
                    w = max_w;
                }
            }

            // If the difference between values is less than or equal to the
            // smoothing delta keep the greater of the values.
            if let Some(dw) = layout.smoothwidth {
                let diff = w as i32 - prevw as i32;
                if diff.abs() as u32 <= dw {
                    w = std::cmp::max(w, prevw);
                }
            }

            // set the new width;
            b.width = w;
        }
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

        let outputs = vec!["eDP1"];
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
    display: glium::backend::glutin_backend::GlutinFacade,
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

        let window = builder.build_glium().unwrap();
        let dloop = &mut DisplayLoop{
            display: window,
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
        let proxy =  self.display.get_window().unwrap().create_window_proxy();

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
                    event.clone(), &self.display
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
    width_update: Option<f64>,
}

impl UpdateConfig {

    pub fn new() -> Self {
        UpdateConfig{
            needs_update: false,
            width_update: None,
            last_update: std::time::Instant::now(),
        }
    }

    pub fn since_last_update(self) -> Duration {
        let now = std::time::Instant::now();
        now.duration_since(self.last_update)
    }

    pub fn update(mut self) -> Self {
        self.needs_update = true;
        self
    }

    pub fn apply_width(mut self, width: Option<f64>) -> Self {
        self.width_update = width;
        self
    }

    fn updated(mut self) -> Self {
        self.last_update = std::time::Instant::now();
        self
    }
}


pub struct UiLoop {
    pub ui: conrod::Ui,
    pub display_info: DisplayInfo,
    pub output: String,
    binders: Vec<Binder>,
    rx: mpsc::Receiver<DispResponse>,
    tx: mpsc::Sender<UiRequest>,
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
                        binders: Vec::new(),
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

    pub fn bind<F>(&mut self, layout: super::Layout, bind: F) -> &Self
        where F: 'static + Send + Fn(Id, &mut UiCell, UpdateConfig) -> UpdateConfig
    {
        let id = self.gen_id();

        self.binders.push(Binder{
            bind: Box::new(bind),
            id: id,
            width: 0,
            layout: layout,
            update: UpdateConfig::new(),
        });

        self
    }

    fn make_sep(slot_id: Id, sep_id: Id) -> Binder {
        Binder{
            bind: Box::new(move |slot_id, mut ui_widgets, mut update| {
                Sep::new()
                    .wh_of(slot_id)
                    .middle_of(slot_id)
                    .set(sep_id, ui_widgets);

                let maybe_drag = ui_widgets.widget_input(sep_id).drags().last();
                if let Some(Drag{total_delta_xy: xy, ..}) = maybe_drag {
                    update = update.apply_width(Some(xy[1]));
                }

                update
            }),
            id: slot_id,
            width: 0,
            layout: super::Layout::new().with_width(Some(DEFAULT_SEP_WIDTH)),
            update: UpdateConfig::new(),
        }
    }

    fn process_ui<F, T>(mut self,
                     ui_renderer: Arc<F>,
                     app_tx: mpsc::Sender<T>)
        where F: 'static + Sync + Send + Fn(&mut UiLoop, mpsc::Sender<T>),
              T: Sync + Send
    {

        // Write the requested widths into a section array. These widths
        // will be configurable but for now set to a default.
        {
            ui_renderer(&mut self, app_tx);
        }

        let master_id;
        let spacer_id;
        let mut binders = Vec::new();
        let mut left_i = 0;
        {
            let mut gid = self.ui.widget_id_generator();

            master_id = gid.next();
            spacer_id = gid.next();

            // insert a seperator between each widget and sort widgets so that
            // Lefts grow inward and Rights grow inward. There are no seperators
            // on the ends and there is a double seperator in the middle where the
            // spacer will be inserted. variable left_i && spacer_i points here
            // [l1 s l2 s s r2 s r1]
            for b in self.binders {
                match b.layout.orientation {
                    super::Orientation::Left => {
                        binders.insert(left_i, b);
                        binders.insert(
                            left_i + 1,
                            UiLoop::make_sep(gid.next(), gid.next()),
                        );
                        left_i += 2;
                    },
                    super::Orientation::Right => {
                        binders.insert(
                            left_i,
                            UiLoop::make_sep(gid.next(), gid.next()),
                        );
                        binders.insert(left_i + 1, b);
                    },
                }
            }
        }

        let spacer_i = left_i;
        let mut needs_update = true;

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
            if events.is_empty() || !needs_update {
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

            needs_update = false;

            // Input each event into the `Ui`.
            for event in events {
                self.ui.handle_event(event);
                needs_update = true;
            }

            let bar_w = match self.ui.w_of(master_id) {
                Some(w) => w  as u32,
                None => self.display_info.width,
            };

            binders.update_widths(&self.ui);
            let totalw = binders.total_width();

            let mut spacer_w = 0;
            if totalw < bar_w {
                spacer_w = bar_w - totalw;
            } else {
                // not implemented - we need to start chopping down size
                // of widgets.
            }

            let mut splits = Vec::with_capacity(binders.len() + 1); // + spacer

            for &Binder{width: w, id, ..} in binders.iter() {
                splits.push((id, Canvas::new().length(w as f64)));
            }

            splits.insert(
                spacer_i, (spacer_id, Canvas::new().length(spacer_w as f64))
            );

            {
                let mut ui = &mut self.ui.set_widgets();
                Canvas::new().flow_right(&splits).set(master_id, ui);

                for b in binders.iter_mut() {
                    let &mut Binder{ref bind, id, mut update, ..} = b;
                    update = bind(id, ui, update);

                    // if any widget needs to be updated we rerender all
                    if update.needs_update {
                        needs_update = true;
                        update.needs_update = false;
                    }

                    if let Some(dw) = update.width_update {
                        println!("{}", dw);
                        if let Some(w) = b.layout.width {
                            let mut neww = w as f64 + dw;
                            if neww < 0. {
                                neww = 0.;
                            }
                            b.layout = b.layout.with_width(Some(neww as u32));
                        }

                        update.width_update = None;
                    }

                    b.update = update;
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

            for &mut Binder{update, ..} in binders.iter_mut() {
                update.updated();
            }
        }

    }
}
