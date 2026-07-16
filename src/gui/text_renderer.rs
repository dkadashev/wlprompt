// SPDX-License-Identifier:  GPL-3.0-or-later
//! Rasterize the passed text into a collection of pixels. Each pixel contains just the opacity
//! info, applying colors is out of the scope of this module.
use std::fs::File;
use std::io::Read as _;

use anyhow::Context as _;
use fontconfig;
use fontdue;

/// A simple class that rasterizes the text.
pub struct TextRenderer {
    fonts: Vec<fontdue::Font>,
    layout: fontdue::layout::Layout, // reused for performance reasons
}

impl TextRenderer {
    pub fn new(font_name: &str) -> anyhow::Result<Self> {
        Ok(TextRenderer {
            fonts: vec![load_font(font_name)?],
            layout: fontdue::layout::Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown),
        })
    }

    /// Rasterize the passed text.
    ///
    /// Returns an iterator over `(x, y, alpha)` tuples, where `x` and `y` denote pixel coordinates
    /// relative to the topmost left corner of the rendered text, and alpha is basically that
    /// pixel's transparency. Dealing with the colors it out of the scope of this function. Fully
    /// transparent pixels are not included into the output.
    pub fn render(&mut self, text: &str, font_size: f32) -> impl Iterator<Item = (u32, u32, u8)> {
        self.layout.clear();
        let style = fontdue::layout::TextStyle::new(text, font_size, 0);
        self.layout.append(&self.fonts, &style);
        self.layout.glyphs().iter().flat_map(|glyph| {
            let (_, cov) = self.fonts[0].rasterize_config(glyph.key);
            // glyph.x & glyph.y are floats, but are always whole numbers, and should be always
            // positive. So the conversions below are fine. But clippy doesn't know that
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_sign_loss)]
            cov.into_iter()
                .enumerate()
                .filter(|(_idx, alpha)| *alpha != 0)
                .map(|(idx, alpha)| {
                    let x = glyph.x as u32 + (idx % glyph.width) as u32;
                    let y = glyph.y as u32 + (idx / glyph.width) as u32;
                    (x, y, alpha)
                })
        })
    }
}

fn load_font(font_name: &str) -> anyhow::Result<fontdue::Font> {
    let fc = fontconfig::Fontconfig::new().context("failed to load fontconfig")?;
    let font = fc
        .find(font_name, None)
        .with_context(|| format!("failed to find font '{font_name}'"))?;

    let mut font_file =
        File::open(font.path).with_context(|| format!("failed to open font '{font_name}'"))?;
    let mut font_buffer = Vec::with_capacity(512 * 1024);
    font_file
        .read_to_end(&mut font_buffer)
        .with_context(|| format!("failed to load font '{font_name}'"))?;
    // The match below wouldn't be necessary if fontdue's error satisfied trait bounds expected by
    // anyhow's with_context(), but it does not, so for now we do this manually
    match fontdue::Font::from_bytes(font_buffer, fontdue::FontSettings::default()) {
        Ok(font) => Ok(font),
        Err(err) => Err(anyhow::format_err!(
            "failed to load font '{font_name}': {err}"
        )),
    }
}
