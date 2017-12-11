
use cable_math::Vec2;

use texture::Texture;

pub struct BitmapFont {
    pub texture: Texture,

    pub first_glyph: u32,
    pub glyph_count: u32,
    pub tile_size: Vec2<u32>,
    pub tile_count: Vec2<u32>,
    pub unkown_glyph_substitute: u32,

    pub char_size: Vec2<u32>,
}

impl BitmapFont {
    /// Passes pairs of positions and uv coordinates to the callback. Three pairs are one triangle,
    /// two triangles form one glyph.
    pub fn cache<F>(
        &mut self,
        text: &str,
        mut offset: Vec2<f32>,
        mut callback: F,
    )
      where F: FnMut(Vec2<f32>, Vec2<f32>),
    {
        offset.y -= self.char_size.y as f32;

        for c in text.chars() {
            let c = c as u32;
            let index: u32;
            if c >= self.first_glyph && c < self.first_glyph + self.glyph_count {
                index = c - self.first_glyph;
            } else {
                index = self.unkown_glyph_substitute;
            }

            let uv_size = Vec2::new(
                self.tile_size.x as f32 / self.texture.width as f32,
                self.tile_size.y as f32 / self.texture.height as f32,
            );
            let uv = Vec2::new(
                (index%self.tile_count.x) as f32 * uv_size.x,
                (index/self.tile_count.x) as f32 * uv_size.y,
            );

            let size = self.tile_size.as_f32();

            callback(offset + Vec2::new(0.0, 0.0),       uv + Vec2::new(0.0, 0.0));
            callback(offset + Vec2::new(size.x, 0.0),    uv + Vec2::new(uv_size.x, 0.0));
            callback(offset + Vec2::new(size.x, size.y), uv + Vec2::new(uv_size.x, uv_size.y));

            callback(offset + Vec2::new(0.0, 0.0),       uv + Vec2::new(0.0, 0.0));
            callback(offset + Vec2::new(size.x, size.y), uv + Vec2::new(uv_size.x, uv_size.y));
            callback(offset + Vec2::new(0.0, size.y),    uv + Vec2::new(0.0, uv_size.y));

            offset.x += self.char_size.x as f32;
        }
    }
}
