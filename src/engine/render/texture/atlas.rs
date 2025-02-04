use crate::engine::ResourceLocation;
use std::sync::Arc;
use rectangle_pack::{pack_rects, GroupedRectsToPlace, RectToInsert, TargetBin};

pub struct TextureAtlas {
    texture: Arc<wgpu::Texture>,
    packer: rectangle_pack::PackedLocation,
}

impl TextureAtlas {
    pub fn make_atlas(device: &wgpu::Device, data: &[(ResourceLocation, image::RgbaImage)]) -> Self {
        // let mut rects_to_pack = GroupedRectsToPlace::new();
        // for x in data {
        //     rects_to_pack.push_rect(&x.0.id, None, RectToInsert::new(x.1.width(), x.1.height(), 1));
        // }
        // let mut bins = TargetBin::new(4096, 4096, 1);
        unimplemented!("HELPME--");
        

        
    }
}