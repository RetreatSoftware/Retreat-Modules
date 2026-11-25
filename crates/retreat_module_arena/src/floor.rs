//! The floor's tiled surface: a checkerboard, so distance and speed can be judged.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Tiles along each side of the floor (3 m tiles on the 60 m arena).
const TILES: u32 = 20;
/// The two tile colors, sRGB.
const LIGHT: [u8; 4] = [176, 174, 168, 255];
const DARK: [u8; 4] = [146, 148, 150, 255];

/// One pixel per tile, drawn without blending between pixels.
pub(crate) fn floor_tiles() -> Image {
    let data: Vec<u8> = (0..TILES * TILES)
        .flat_map(|index| {
            if (index % TILES + index / TILES).is_multiple_of(2) {
                LIGHT
            } else {
                DARK
            }
        })
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: TILES,
            height: TILES,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}
