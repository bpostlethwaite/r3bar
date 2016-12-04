#[derive(Debug, Clone, Copy)]
pub struct Animate {
    max_frames: i64,
    frames: i64,
}

impl Animate {

    pub fn new() -> Animate {
        Animate{frames: 0, max_frames: 4294967296}
    }

    pub fn next_frame(&mut self) {
        if self.frames >= self.max_frames {
            self.frames = 0;
        } else {
            self.frames = self.frames + 1;
        }
    }

    pub fn tick(&self, frames_per_tick: i64) -> bool {
        (self.frames % frames_per_tick) == 0
    }
}
