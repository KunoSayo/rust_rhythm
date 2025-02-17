use egui::{Pos2, Rect};
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

pub fn map_point_to_std_pos_in_rect(rect: &Rect, pos: Pos2) -> (f32, f32) {
    let x = (pos.x - rect.center().x) * 2.0 / rect.width();
    let y = (rect.center().y - pos.y) * 2.0 / rect.height();
    (x, y)
}

#[cfg(test)]
mod test {
    use crate::state::editor::util::map_point_to_std_pos_in_rect;
    use egui::{Pos2, Rect};

    #[test]
    fn test_map() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1920.0, 1080.0));

        assert_eq!(
            map_point_to_std_pos_in_rect(&rect, Pos2::new(0.0, 0.0)),
            (-1.0, 1.0)
        );
        assert_eq!(
            map_point_to_std_pos_in_rect(&rect, Pos2::new(1920.0, 0.0)),
            (1.0, 1.0)
        );
        assert_eq!(
            map_point_to_std_pos_in_rect(&rect, Pos2::new(0.0, 1080.0)),
            (-1.0, -1.0)
        );
        assert_eq!(
            map_point_to_std_pos_in_rect(&rect, Pos2::new(1920.0, 1080.0)),
            (1.0, -1.0)
        );
        assert_eq!(
            map_point_to_std_pos_in_rect(&rect, Pos2::new(1920.0 / 2.0, 1080.0 / 2.0)),
            (0.0, 0.0)
        );
    }
}
