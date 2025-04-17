use crate::engine::{ResourceLocation, TextureWrapper};
use anyhow::anyhow;
use egui::ahash::HashMap;
use egui::{Pos2, Rect, Vec2};
use image::{ColorType, DynamicImage, GenericImage};
use nalgebra::Vector2;
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, RectToInsert,
    TargetBin,
};
use std::collections::BTreeMap;
use std::ops::{Add, Sub};
use log::info;
use wgpu::Queue;

#[derive(Debug)]
pub struct TextureAtlas {
    pub texture: TextureWrapper,
    pub locs: HashMap<ResourceLocation, Rect>,
}

impl TextureAtlas {
    /// Return the tex coord for a atlas. left top, right top, left bottom, right bottom.
    pub fn get_tex_coord(&self, loc: &ResourceLocation) -> Option<[Vector2<f32>; 4]> {
        if let Some(loc) = self.locs.get(loc) {
            // let mut loc = *loc;
            // loc.min = loc.min.add(Vec2::new(0.5, 0.5));
            // loc.max = loc.max.sub(Vec2::new(0.5, 0.5));
            let left = loc.left() / 4096.0;
            let top = loc.top() / 4096.0;
            let right = loc.right() / 4096.0;
            let bottom = loc.bottom() / 4096.0;

            return Some([
                Vector2::new(left, top),
                Vector2::new(right, top),
                Vector2::new(left, bottom),
                Vector2::new(right, bottom),
            ]);
        }
        None
    }

    pub fn get_tex_coord_left_right_slice(
        &self,
        loc: &ResourceLocation,
        left_pixel: f32,
        right_pixel: f32,
    ) -> Option<[[Vector2<f32>; 4]; 3]> {
        if let Some(loc) = self.locs.get(loc) {
            // let mut loc = *loc;
            // loc.min = loc.min.add(Vec2::new(0.5, 0.5));
            // loc.max = loc.max.sub(Vec2::new(0.5, 0.5));
            let left = loc.left() / 4096.0;
            let top = loc.top() / 4096.0;
            let right = loc.right() / 4096.0;
            let bottom = loc.bottom() / 4096.0;

            let left_right = (loc.left() + left_pixel) / 4096.0;
            let right_left = (loc.right() - right_pixel) / 4096.0;

            return Some([
                [
                    Vector2::new(left, top),
                    Vector2::new(left_right, top),
                    Vector2::new(left, bottom),
                    Vector2::new(left_right, bottom),
                ],
                [
                    Vector2::new(left_right, top),
                    Vector2::new(right_left, top),
                    Vector2::new(left_right, bottom),
                    Vector2::new(right_left, bottom),
                ],
                [
                    Vector2::new(right_left, top),
                    Vector2::new(right, top),
                    Vector2::new(right_left, bottom),
                    Vector2::new(right, bottom),
                ],
            ]);
        }
        None
    }

    pub fn make_atlas(
        device: &wgpu::Device,
        queue: &Queue,
        data: &[(ResourceLocation, &DynamicImage)],
    ) -> anyhow::Result<Self> {
        let mut rects_to_pack = GroupedRectsToPlace::<_, ()>::new();
        for x in data {
            rects_to_pack.push_rect(&x.0, None, RectToInsert::new(x.1.width(), x.1.height(), 1));
        }
        let mut bins = BTreeMap::new();

        bins.insert((), TargetBin::new(4096, 4096, 1));

        let result = pack_rects(
            &rects_to_pack,
            &mut bins,
            &volume_heuristic,
            &contains_smallest_box,
        )
        .map_err(|e| anyhow!("Not enough space"))?;

        let mut locs = HashMap::default();
        for (id, ((), loc)) in result.packed_locations() {
            locs.insert(
                (*id).clone(),
                Rect::from_min_size(
                    Pos2::new(loc.x() as f32, loc.y() as f32),
                    Vec2::new(loc.width() as f32, loc.height() as f32),
                ),
            );
        }

        let mut img = DynamicImage::new(4096, 4096, ColorType::Rgba8);

        for x in data {
            let loc = locs.get(&x.0).unwrap();
            img.copy_from(x.1, loc.left() as u32, loc.top() as u32);
        }

        let texture = TextureWrapper::from_image(device, queue, &img, Some("Texture Atlas"))?;

        info!("Got atlas locs {:?}", locs);
        Ok(Self { texture, locs })
    }
}
