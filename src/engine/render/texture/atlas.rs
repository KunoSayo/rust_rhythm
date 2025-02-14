use crate::engine::{ResourceLocation, TextureWrapper};
use anyhow::anyhow;
use egui::ahash::HashMap;
use image::{ColorType, DynamicImage, GenericImage};
use rectangle_pack::{contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, RectToInsert, TargetBin};
use std::collections::BTreeMap;
use wgpu::Queue;

#[derive(Debug)]
pub struct TextureAtlas {
    texture: TextureWrapper,
    locs: HashMap<ResourceLocation, (u32, u32)>,
}

impl TextureAtlas {
    pub fn make_atlas(device: &wgpu::Device, queue: &Queue, data: &[(ResourceLocation, &DynamicImage)]) -> anyhow::Result<Self> {
        let mut rects_to_pack = GroupedRectsToPlace::<_, ()>::new();
        for x in data {
            rects_to_pack.push_rect(&x.0, None, RectToInsert::new(x.1.width(), x.1.height(), 1));
        }
        let mut bins = BTreeMap::new();

        bins.insert((), TargetBin::new(4096, 4096, 1));


        let result = pack_rects(&rects_to_pack, &mut bins, &volume_heuristic, &contains_smallest_box)
            .map_err(|e| anyhow!("Not enough space"))?;


        let mut locs = HashMap::default();
        for (id, ((), loc)) in result.packed_locations() {
            locs.insert((*id).clone(), (loc.x(), loc.y()));
        }

        let mut img = DynamicImage::new(4096, 4096, ColorType::Rgba8);

        for x in data {
            let loc = locs.get(&x.0).unwrap();
            img.copy_from(x.1, loc.0, loc.1);
        }

        let texture = TextureWrapper::from_image(device, queue, &img, Some("Texture Atlas"))?;

        Ok(Self {
            texture,
            locs,
        })
    }
}