use winit::event_loop::{EventLoop, EventLoopBuilder};

use crate::engine::window::{EventLoopMessage, WindowManager};
use crate::state::{InitState, MenuState};

mod engine;
mod state;

pub fn real_main() {
    _main(EventLoop::with_user_event().build().unwrap());
}

fn _main(event_loop: EventLoop<EventLoopMessage>) {
    println!("[Std Stream] Joined the real main");
    eprintln!("[Err Stream] Joined the real main");
    log::info!("[Log Info] Joined the real main");
    let is_3d = std::env::var("3d").map(|x| x == "1").unwrap_or(true);

    log::info!("Got the window");

    match WindowManager::new(&event_loop) {
        Ok(am) => {
            log::info!("Got the main application");
            am.run_loop(event_loop, InitState::new(Box::new(MenuState::new())));
        }
        Err(e) => {
            log::error!("Init the app manager failed for {:?}", e);
            eprintln!("Init the app manager failed for {:?}", e);
        }
    }
}


#[no_mangle]
#[cfg(feature = "android")]
#[cfg(target_os = "android")]
pub fn android_main(app: android_activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    use winit::event_loop::EventLoopBuilder;

    std::env::set_var("RUST_BACKTRACE", "full");

    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Trace));
    let el = EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build();
    _main(el);
}
