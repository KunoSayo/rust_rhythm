use crate::engine::renderer::texture_renderer::{FragUniform, TextureObject, TextureRenderer};
use crate::engine::uniform::{create_static_uniform_buffer, uniform_bind_buffer_entry};
use crate::engine::{MainRendererData, ResourceLocation, ResourceManager, WgpuData};
use crate::game::note::consts::NOTE_HEIGHT_PIXEL;
use crate::game::note::Note;
use egui::Rect;
use nalgebra::{Vector2, Vector4};
use std::iter::once;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, Buffer,
    CommandEncoderDescriptor, Device,
};

pub struct NoteRenderDesc {
    // Left, Middle, Right
    pub tex_coords: [[Vector2<f32>; 4]; 3],
    pub left_pixel: u32,
    pub right_pixel: u32,
    /// Half height in pixel
    pub note_half_height: f32,
}

impl NoteRenderDesc {
    pub fn get_note_render_obj<T: Note>(
        &self,
        viewport_size: (f32, f32),
        note_center_y: f32,
        note: &T,
        mut consume: impl FnMut(TextureObject),
    ) {
        let left_x = note.get_x() - note.get_width() / 2.0;
        let right_x = note.get_x() + note.get_width() / 2.0;
        let left_side_right_x = left_x + (self.left_pixel as f32) * 2.0 / viewport_size.0;
        let right_side_left_x = right_x - (self.right_pixel as f32) * 2.0 / viewport_size.0;
        let up = note_center_y + self.note_half_height * 2.0 / viewport_size.1;
        let down = note_center_y - self.note_half_height * 2.0 / viewport_size.1;

        // eprintln!("Got note situation: {} {} {} {} and up down {} {}", left_x, left_side_right_x, right_side_left_x, right_x, up, down);

        consume(TextureObject::new_rect(
            Vector2::new(left_x, up),
            Vector2::new(left_side_right_x, down),
            &self.tex_coords[0],
        ));
        consume(TextureObject::new_rect(
            Vector2::new(left_side_right_x, up),
            Vector2::new(right_side_left_x, down),
            &self.tex_coords[1],
        ));
        consume(TextureObject::new_rect(
            Vector2::new(right_side_left_x, up),
            Vector2::new(right_x, down),
            &self.tex_coords[2],
        ));
    }

    pub fn new(
        tex_coords: [[Vector2<f32>; 4]; 3],
        left_pixel: u32,
        right_pixel: u32,
        note_half_height: f32,
    ) -> Self {
        Self {
            tex_coords,
            left_pixel,
            right_pixel,
            note_half_height,
        }
    }
}

pub struct NoteRenderer {
    pub gray_tint_buffer: Buffer,
    pub gray_tint_bg: BindGroup,
    pub atlas_group: BindGroup,
    pub normal_note: NoteRenderDesc,
    pub objs: Vec<TextureObject>,
    pub background_objs: Vec<TextureObject>,
}

impl NoteRenderer {
    pub fn render(
        &mut self,
        gpu: &WgpuData,
        render: &mut MainRendererData,
        tr: &TextureRenderer,
        vp: &Rect,
    ) {
        let device = &gpu.device;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

        tr.render(
            device,
            &mut encoder,
            &gpu.data,
            &mut render.staging_belt,
            &self.atlas_group,
            &self.gray_tint_bg,
            &self.background_objs,
            &gpu.views.get_screen().view,
            vp,
        );
        tr.render(
            device,
            &mut encoder,
            &gpu.data,
            &mut render.staging_belt,
            &self.atlas_group,
            &tr.white_tint_bg,
            &self.objs,
            &gpu.views.get_screen().view,
            vp,
        );
        render.staging_belt.finish();
        gpu.queue.submit(once(encoder.finish()));
        render.staging_belt.recall();

        self.objs.clear();
        self.background_objs.clear();
    }
    pub fn new(device: &Device, tr: &TextureRenderer, rs: &ResourceManager) -> Self {
        let gray_tint_buffer = create_static_uniform_buffer(
            device,
            &[FragUniform {
                tint: Vector4::new(0.5, 0.5, 0.5, 1.0),
            }],
        );
        let atlas = rs
            .atlas
            .get(&ResourceLocation::from_name("default"))
            .expect("Where is my atlas");

        let atlas_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &tr.texture_bind_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&atlas.texture.view),
            }],
        });

        let gray_tint_bg = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &tr.tint_bind_layout,
            entries: &[uniform_bind_buffer_entry(0, &gray_tint_buffer)],
        });

        let normal_tex_coords = atlas
            .get_tex_coord_left_right_slice(&ResourceLocation::from_name("note"), 16.0, 16.0)
            .expect("Failed to get normal note tex coords");
        Self {
            gray_tint_buffer,
            gray_tint_bg,
            atlas_group,
            normal_note: NoteRenderDesc {
                tex_coords: normal_tex_coords,
                left_pixel: 16,
                right_pixel: 16,
                note_half_height: NOTE_HEIGHT_PIXEL / 2.0,
            },
            objs: vec![],
            background_objs: vec![],
        }
    }
}
