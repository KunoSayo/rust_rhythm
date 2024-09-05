// #![windows_subsystem = "windows"]

use log::LevelFilter;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();
    rr_core::real_main();
}
