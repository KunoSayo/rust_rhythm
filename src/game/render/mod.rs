use crate::engine::renderer::texture_renderer::TextureObject;
use crate::game::note::Note;
use nalgebra::Vector2;

pub struct NoteRenderDesc {
    // Left, Middle, Right
    pub tex_coords: [[Vector2<f32>; 4]; 3],
    pub left_pixel: u32,
    pub right_pixel: u32,
    pub note_half_height: f32,
}

impl NoteRenderDesc {
    pub fn get_note_render_obj<T: Note>(&self, viewport_size: (f32, f32), center_y: f32, note: &T, consume: impl Fn(TextureObject)) {
        let left_x = note.get_x() - note.get_width();
        let right_x = note.get_x() - note.get_width();
        let left_side_right_x = left_x + (self.left_pixel as f32) / viewport_size.0;
        let right_side_left_x = right_x - (self.right_pixel as f32) / viewport_size.0;
        let up = center_y + self.note_half_height / viewport_size.1;
        let down = center_y - self.note_half_height / viewport_size.1;

        consume(TextureObject::new_rect(Vector2::new(left_x, up), Vector2::new(left_side_right_x, down), &self.tex_coords[0]));
        consume(TextureObject::new_rect(Vector2::new(left_side_right_x, up), Vector2::new(right_side_left_x, down), &self.tex_coords[1]));
        consume(TextureObject::new_rect(Vector2::new(right_side_left_x, up), Vector2::new(right_x, down), &self.tex_coords[2]));
    }
}