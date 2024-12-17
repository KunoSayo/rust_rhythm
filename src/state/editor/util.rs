use std::path::PathBuf;
use winit::window::Window;

pub fn select_music_file(window: &Window) -> Option<PathBuf> {
    let result = rfd::FileDialog::new()
        .add_filter("music", &["mp3", "ogg"])
        .set_directory("/")
        .set_parent(window)
        .pick_file();

    log::info!("Select music result: {result:?}");

    result
}