
use crate::engine::global::STATIC_DATA;
use crate::engine::manager::{AsyncWindowManager, WindowManager, WinitEventLoopMessage};
use crate::state::{InitState, MenuState};
use winit::event_loop::EventLoop;

mod engine;
mod game;
mod state;
mod ui;

pub fn real_main() {
    _main(EventLoop::with_user_event().build().unwrap());
}

fn _main(event_loop: EventLoop<WinitEventLoopMessage>) {
    println!("[Std Stream] Joined the real main");
    eprintln!("[Err Stream] Joined the real main");
    log::info!("[Log Info] Joined the real main");
    let is_3d = std::env::var("3d").map(|x| x == "1").unwrap_or(true);

    log::info!("Got the window");

    // let elp = event_loop.create_proxy();
    // match WindowManager::new(elp) {
    //     Ok(am) => {
    //         log::info!("Got the main application");
    //         am.run_loop(event_loop, InitState::new(Box::new(MenuState::new())));
    //         STATIC_DATA.cfg_data.write().unwrap().check_save();
    //     }
    //     Err(e) => {
    //         log::error!("Init the app manager failed for {:?}", e);
    //         eprintln!("Init the app manager failed for {:?}", e);
    //     }
    // }

    let start = InitState::new(Box::new(MenuState::new()));
    match AsyncWindowManager::new(&event_loop, start) {
        Ok(awm) => {
            log::info!("Got the async window manager.");
            awm.run_loop(event_loop);
            STATIC_DATA.cfg_data.write().unwrap().check_save();
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
    use winit::event_loop::EventLoopBuilder;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    std::env::set_var("RUST_BACKTRACE", "full");

    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Trace));
    let el = EventLoopBuilder::with_user_event()
        .with_android_app(app)
        .build();
    _main(el);
}
