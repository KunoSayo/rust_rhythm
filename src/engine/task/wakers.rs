use std::sync::{Arc, Mutex};
use std::task::Wake;

use winit::window::{Window, WindowId};

use crate::engine::manager::{EventLoopMessage, EventLoopProxyType};

pub struct WindowWaker {
    proxy: Mutex<EventLoopProxyType>,
    window_id: WindowId,
}

impl WindowWaker {
    pub fn new(proxy: EventLoopProxyType, window: &Window) -> Arc<Self> {
        Self {
            proxy: Mutex::new(proxy),
            window_id: window.id(),
        }.into()
    }
}


impl Wake for WindowWaker {
    fn wake(self: Arc<Self>) {
        let _ = self.proxy.lock().expect("Get proxy lock failed")
            .send_event(EventLoopMessage::WakeUp(self.window_id));
    }
}

pub struct NeverWaker;

impl Wake for NeverWaker {
    fn wake(self: Arc<Self>) {}
}
