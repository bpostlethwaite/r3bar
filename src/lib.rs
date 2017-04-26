#[macro_use] extern crate conrod;
extern crate byteorder;
extern crate chrono;
extern crate i3ipc;
extern crate regex;
extern crate serde_json;
extern crate unix_socket;
extern crate image;

pub mod message;
pub mod widgets;
pub mod bar;
pub mod error;
pub mod gauges;
pub mod r3ipc;
pub mod sensors;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Orientation {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    width: Option<u32>,
    minwidth: Option<u32>,
    maxwidth: Option<u32>,
    smoothwidth: Option<u32>,
    orientation: Orientation,
}

impl Layout {
    pub fn new() -> Self {
        Layout{
            width: None,
            minwidth: None,
            maxwidth: None,
            smoothwidth: Some(4),
            orientation: Orientation::Right,
        }
    }

    pub fn with_width(mut self, width: Option<u32>) -> Self {
        self.width = width;
        self
    }

    pub fn with_minwidth(mut self, width: Option<u32>) -> Self {
        self.minwidth = width;
        self
    }

    pub fn with_orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }
}
