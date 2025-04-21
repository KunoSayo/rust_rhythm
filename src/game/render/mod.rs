use crate::engine::renderer::texture_renderer::{FragUniform, TextureObject, TextureRenderer};
use crate::engine::uniform::{create_static_uniform_buffer, uniform_bind_buffer_entry};
use crate::engine::{MainRendererData, ResourceLocation, ResourceManager, WgpuData};
use crate::game::beatmap::play::{NoteHitResult, NoteResult, PlayingNote};
use crate::game::note::consts::NOTE_HEIGHT_PIXEL;
use crate::game::note::{Note, NoteHitType};
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
    pub slide_coords: [[Vector2<f32>; 4]; 3],
    // [[Left, Middle, Right]; top, mid, bottom]
    // [top/mid/bottom][left/mid/right]
    pub long_coords: [[[Vector2<f32>; 4]; 3]; 3],
    pub left_pixel: u32,
    pub right_pixel: u32,
    /// Half height in pixel
    pub note_half_height: f32,
}

impl NoteRenderDesc {
    fn get_obj(
        &self,
        left_x: f32,
        right_x: f32,
        up: f32,
        down: f32,
        viewport_size: (f32, f32),
        tex_coords: &[[Vector2<f32>; 4]; 3],
        consume: &mut impl FnMut(TextureObject),
    ) {
        let left_side_right_x = left_x + (self.left_pixel as f32) * 2.0 / viewport_size.0;
        let right_side_left_x = right_x - (self.right_pixel as f32) * 2.0 / viewport_size.0;

        // eprintln!("Got note situation: {} {} {} {} and up down {} {}", left_x, left_side_right_x, right_side_left_x, right_x, up, down);

        consume(TextureObject::new_rect(
            Vector2::new(left_x, up),
            Vector2::new(left_side_right_x, down),
            &tex_coords[0],
        ));
        consume(TextureObject::new_rect(
            Vector2::new(left_side_right_x, up),
            Vector2::new(right_side_left_x, down),
            &tex_coords[1],
        ));
        consume(TextureObject::new_rect(
            Vector2::new(right_side_left_x, up),
            Vector2::new(right_x, down),
            &tex_coords[2],
        ));
    }

    /// `start_secs`: The time for y = 0  
    /// `time_scale`: the y delta if time delta is 1.0s
    pub fn get_note_render_obj<T: Note>(
        &self,
        viewport_size: (f32, f32),
        center_secs: f32,
        time_scale: f32,
        note: &T,
        mut consume: impl FnMut(TextureObject),
    ) {
        let left_x = note.get_x() - note.get_width() / 2.0;
        let right_x = note.get_x() + note.get_width() / 2.0;

        let center_y = ((note.get_time() as f32 / 1000.0) - center_secs) * time_scale;
        if let Some(et) = note.get_end_time() {
            // we are long note.
            // getting bottom obj to render.
            let up = center_y + self.note_half_height * 2.0 / viewport_size.1;
            let down = center_y - self.note_half_height * 2.0 / viewport_size.1;
            let mid_down = center_y;
            self.get_obj(
                left_x,
                right_x,
                up,
                down,
                viewport_size,
                &self.long_coords[2],
                &mut consume,
            );
            let ender_time_center_y = ((et as f32 / 1000.0) - center_secs) * time_scale;
            let up = ender_time_center_y + self.note_half_height * 2.0 / viewport_size.1;
            let down = ender_time_center_y - self.note_half_height * 2.0 / viewport_size.1;
            let mid_up = ender_time_center_y;
            self.get_obj(
                left_x,
                right_x,
                up,
                down,
                viewport_size,
                &self.long_coords[0],
                &mut consume,
            );

            self.get_obj(
                left_x,
                right_x,
                mid_up,
                mid_down,
                viewport_size,
                &self.long_coords[1],
                &mut consume,
            );
        } else {
            let up = center_y + self.note_half_height * 2.0 / viewport_size.1;
            let down = center_y - self.note_half_height * 2.0 / viewport_size.1;
            match note.get_note_type() {
                NoteHitType::Click => {
                    self.get_obj(
                        left_x,
                        right_x,
                        up,
                        down,
                        viewport_size,
                        &self.tex_coords,
                        &mut consume,
                    );
                }
                NoteHitType::Slide => {
                    self.get_obj(
                        left_x,
                        right_x,
                        up,
                        down,
                        viewport_size,
                        &self.slide_coords,
                        &mut consume,
                    );
                }
            }
        }
    }

    pub fn get_note_render_obj_by_y<T: Note>(
        &self,
        viewport_size: (f32, f32),
        note_y: f32,
        note_end_y: f32,
        note: &T,
        mut consume: impl FnMut(TextureObject),
    ) {
        let left_x = note.get_x() - note.get_width() / 2.0;
        let right_x = note.get_x() + note.get_width() / 2.0;

        let note_center_y = note_y;
        if let Some(et) = note.get_end_time() {
            let up = note_center_y + self.note_half_height * 2.0 / viewport_size.1;
            let down = note_center_y - self.note_half_height * 2.0 / viewport_size.1;
            let mid_down = note_center_y;
            self.get_obj(
                left_x,
                right_x,
                up,
                down,
                viewport_size,
                &self.long_coords[2],
                &mut consume,
            );
            let up = note_end_y + self.note_half_height * 2.0 / viewport_size.1;
            let down = note_end_y - self.note_half_height * 2.0 / viewport_size.1;
            let mid_up = note_end_y;
            self.get_obj(
                left_x,
                right_x,
                up,
                down,
                viewport_size,
                &self.long_coords[0],
                &mut consume,
            );

            self.get_obj(
                left_x,
                right_x,
                mid_up,
                mid_down,
                viewport_size,
                &self.long_coords[1],
                &mut consume,
            );
        } else {
            let up = note_center_y + self.note_half_height * 2.0 / viewport_size.1;
            let down = note_center_y - self.note_half_height * 2.0 / viewport_size.1;
            match note.get_note_type() {
                NoteHitType::Click => {
                    self.get_obj(
                        left_x,
                        right_x,
                        up,
                        down,
                        viewport_size,
                        &self.tex_coords,
                        &mut consume,
                    );
                }
                NoteHitType::Slide => {
                    self.get_obj(
                        left_x,
                        right_x,
                        up,
                        down,
                        viewport_size,
                        &self.slide_coords,
                        &mut consume,
                    );
                }
            }
        }
    }

    pub fn new(
        tex_coords: [[Vector2<f32>; 4]; 3],
        slide_coords: [[Vector2<f32>; 4]; 3],
        long_coords: [[[Vector2<f32>; 4]; 3]; 3],
        left_pixel: u32,
        right_pixel: u32,
        note_half_height: f32,
    ) -> Self {
        Self {
            tex_coords,
            slide_coords,
            long_coords,
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
    pub note_desc: NoteRenderDesc,
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
        if vp.is_negative() || vp.any_nan() || vp.area() <= 0.0 || vp.left() < 0.0 || vp.top() < 0.0 {
            return;
        }
        let device = &gpu.device;
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Note Renderer"),
        });

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

        // todo: long texture.
        let long_coords = [
            atlas
                .get_tex_coord_left_right_slice(
                    &ResourceLocation::from_name("long_top"),
                    16.0,
                    16.0,
                )
                .expect("Failed to get long top"),
            atlas
                .get_tex_coord_left_right_slice(
                    &ResourceLocation::from_name("long_mid"),
                    16.0,
                    16.0,
                )
                .expect("Failed to get long mid"),
            atlas
                .get_tex_coord_left_right_slice(
                    &ResourceLocation::from_name("long_bottom"),
                    16.0,
                    16.0,
                )
                .expect("Failed to get long bottom"),
        ];
        let slide_coords = Default::default();
        Self {
            gray_tint_buffer,
            gray_tint_bg,
            atlas_group,
            note_desc: NoteRenderDesc {
                tex_coords: normal_tex_coords,
                slide_coords,
                long_coords,
                left_pixel: 16,
                right_pixel: 16,
                note_half_height: NOTE_HEIGHT_PIXEL / 2.0,
            },
            objs: vec![],
            background_objs: vec![],
        }
    }

    pub fn collect_playing_notes<T: Note>(
        &mut self,
        notes: &[PlayingNote<T>],
        viewport_size: (f32, f32),
        current_y: f32,
    ) {
        let NoteRenderer {
            note_desc: desc,
            objs: fgs,
            background_objs: bgs,
            ..
        } = self;
        let mut to_objs = |obj| {
            fgs.push(obj);
        };
        let mut to_bg_objs = |obj| {
            bgs.push(obj);
        };
        for x in notes {
            if x.start_result.is_none() {
                desc.get_note_render_obj_by_y(
                    viewport_size,
                    x.note_y - current_y,
                    x.note_end_y - current_y,
                    x,
                    &mut to_objs,
                );
            } else {
                if let Some(NoteHitResult {
                    grade: NoteResult::Miss,
                    ..
                }) = x.start_result
                {
                    desc.get_note_render_obj_by_y(
                        viewport_size,
                        0.0,
                        x.note_end_y - current_y,
                        x,
                        &mut to_bg_objs,
                    );
                } else {
                    desc.get_note_render_obj_by_y(
                        viewport_size,
                        0.0,
                        x.note_end_y - current_y,
                        x,
                        &mut to_objs,
                    );
                };
            }
        }
    }
}
