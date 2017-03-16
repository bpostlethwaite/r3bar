use conrod::backend::glium::glium;
use std::borrow::Borrow;
use self::glium::Surface;
use self::glium::glutin;
use self::glium::backend::Context;

use std::rc::Rc;
use std::os::raw::c_void;
use std::cell::{RefCell, Ref};
use std::ops::Deref;


pub struct Backend {
    window: Rc<glutin::Window>,
}

unsafe impl glium::backend::Backend for Backend {
    fn swap_buffers(&self) -> Result<(), glium::SwapBuffersError> {
        match self.window.swap_buffers() {
            Ok(()) => Ok(()),
            Err(glutin::ContextError::IoError(_)) => panic!(),
            Err(glutin::ContextError::ContextLost) => Err(glium::SwapBuffersError::ContextLost),
        }
    }

    // this function is called only after the OpenGL context has been made current
    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        self.window.get_proc_address(symbol) as *const _
    }

    // this function is used to adjust the viewport when the user wants to draw
    // or blit on the whole window
    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        // we default to a dummy value is the window no longer exists
        self.window.get_inner_size().unwrap_or((128, 128))
    }

    fn is_current(&self) -> bool {
        // if you are using a library that doesn't provide an equivalent to
        // `is_current`, you can just put `unimplemented!` and pass `false`
        // when you create the `Context` (see below)
        self.window.is_current()
    }

    unsafe fn make_current(&self) {
        self.window.make_current().unwrap();
    }
}

/// Facade implementation for glutin. Wraps both glium and glutin.
#[derive(Clone)]
pub struct R3Display {
    // contains everything related to the current context and its state
    context: Rc<glium::backend::Context>,

    backend: Rc<glutin::Window>,
}

impl glium::backend::Facade for R3Display {
    #[inline]
    fn get_context(&self) -> &Rc<Context> {
        &self.context
    }
}

impl Deref for R3Display {
    type Target = Context;

    #[inline]
    fn deref(&self) -> &Context {
        &self.context
    }
}

pub struct PollEventsIter<'a> {
    window: &'a Rc<glutin::Window>,
}


/// Blocking iterator over all the events received by the window.
///
/// This iterator polls for events, until the window associated with its context
/// is closed.
pub struct WaitEventsIter<'a> {
    window: &'a Rc<glutin::Window>,
}

impl<'a> Iterator for PollEventsIter<'a> {
    type Item = glutin::Event;

    #[inline]
    fn next(&mut self) -> Option<glutin::Event> {
        self.window.as_ref().borrow().poll_events().next()
    }
}

impl<'a> Iterator for WaitEventsIter<'a> {
    type Item = glutin::Event;

    #[inline]
    fn next(&mut self) -> Option<glutin::Event> {
        self.window.as_ref().borrow().wait_events().next()
    }
}


impl R3Display {

    pub fn new(window: Rc<glutin::Window>) -> Self {
        // now building the context
        let context = unsafe {
            // The first parameter is our backend.
            //
            // The second parameter tells glium whether or not it should
            // regularly call `is_current` on the backend to make sure that
            // the OpenGL context is still the current one.
            //
            // It is recommended to pass `true`, but you can pass `false`
            // if you are sure that no other OpenGL context will be made
            // current in this thread.
            glium::backend::Context::new::<_, ()>(Backend { window: window.clone() },
                                                  true, Default::default())
        }.unwrap();

        R3Display{context: context, backend: window}
    }


    /// Reads all events received by the window.
    ///
    /// This iterator polls for events and can be exhausted.
    #[inline]
    pub fn poll_events(&self) -> PollEventsIter {
        PollEventsIter {
            window: &self.backend,
        }
    }

    /// Reads all events received by the window.
    #[inline]
    pub fn wait_events(&self) -> WaitEventsIter {
        WaitEventsIter {
            window: &self.backend,
        }
    }

    /// Returns the underlying window, or `None` if glium uses a headless context.
    #[inline]
    pub fn get_window(&self) -> &glutin::Window {
        self.backend.borrow()
    }


}
