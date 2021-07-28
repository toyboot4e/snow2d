/*!
Simple text rendering

See [`crate::ui::node::Text`] for rich and retained-mode text rendering.
*/

pub mod font;

pub type Result<T> = std::io::Result<T>;

use rokol::fons::FonsQuad;
use std::{borrow::Cow, fs, io};

use crate::{
    gfx::{batch::QuadData, draw::*, geom2d::Vec2f},
    utils::arena::{Arena, Index},
};

use self::font::*;

/// Handle of a specific font
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontHandle {
    ix: FontIx,
}

/// Internal utilities
impl FontHandle {
    pub fn from_ix(ix: FontIx) -> Self {
        Self { ix }
    }

    /// Actually font index for rendering
    pub fn font_ix(&self) -> FontIx {
        self.ix
    }
}

/// Fontface for a font family
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFace {
    Regular,
    Bold,
    Italic,
}

/// Set of [`FontHandle`] that covers one family
#[derive(Debug, Clone)]
pub struct FontFamilyHandle {
    name: String,
    regular: FontHandle,
    bold: Option<FontHandle>,
    italic: Option<FontHandle>,
}

impl FontFamilyHandle {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn regular(&self) -> FontHandle {
        self.regular
    }

    pub fn bold(&self) -> FontHandle {
        self.bold.unwrap_or(self.regular)
    }

    pub fn italic(&self) -> FontHandle {
        self.italic.unwrap_or(self.regular)
    }
}

/// Bundle of font texture and font storage
///
/// Currently only TTF is supported.
///
/// * TODO: Bitmap support
/// * TODO: SDF support
#[derive(Debug)]
pub struct FontBook {
    /// Texture where font data is rasterized
    pub tex: FontTexture,
    /// Loaded font handles
    pub families: Arena<FontFamilyHandle>,
}

impl FontBook {
    /// Loads a set of fonts from storage
    pub fn load_family(&mut self, font_family: &FontFamilyDesc) -> Result<Index<FontFamilyHandle>> {
        let set = FontFamilyHandle {
            name: font_family.name.clone(),
            regular: self.load_font(&font_family.regular)?,
            bold: if let Some(font) = &font_family.bold {
                Some(self.load_font(font)?)
            } else {
                None
            },
            italic: if let Some(font) = &font_family.italic {
                Some(self.load_font(font)?)
            } else {
                None
            },
        };

        let key = self.families.insert(set);

        Ok(key)
    }

    fn load_font(&mut self, font: &FontDesc) -> Result<FontHandle> {
        let mem: Cow<[u8]> = match &font.load {
            LoadDesc::Path(p) => {
                let x = fs::read(p)?;
                Cow::Owned(x)
            }
            LoadDesc::Mem(m) => Cow::Borrowed(m),
        };

        let ix = self
            .tex
            .add_font_mem(&font.name, mem.as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        log::trace!("font {:?} loaded at index `{:?}`", font.name, ix);

        Ok(FontHandle { ix })
    }
}

// --------------------------------------------------------------------------------
// Immediate-mode rendering procedures

/// Sets a text quad
#[inline]
pub fn set_text_quad(q: &mut QuadData, fons_quad: &FonsQuad, base_pos: Vec2f, color: [u8; 4]) {
    // NOTE: quad = [left_up, right_up, left_down, right_down]
    q[0].uv = [fons_quad.s0, fons_quad.t0];
    q[1].uv = [fons_quad.s1, fons_quad.t0];
    q[2].uv = [fons_quad.s0, fons_quad.t1];
    q[3].uv = [fons_quad.s1, fons_quad.t1];

    q[0].pos = [
        fons_quad.x0 as f32 + base_pos.x,
        fons_quad.y0 as f32 + base_pos.y,
    ];
    q[1].pos = [
        fons_quad.x1 as f32 + base_pos.x,
        fons_quad.y0 as f32 + base_pos.y,
    ];
    q[2].pos = [
        fons_quad.x0 as f32 + base_pos.x,
        fons_quad.y1 as f32 + base_pos.y,
    ];
    q[3].pos = [
        fons_quad.x1 as f32 + base_pos.x,
        fons_quad.y1 as f32 + base_pos.y,
    ];

    q[0].color = color;
    q[1].color = color;
    q[2].color = color;
    q[3].color = color;
}

/// Sets a text quad and a shadow quad of it
#[inline]
pub fn set_text_quad_with_shadow(
    quads: &mut impl QuadIter,
    img: rokol::gfx::Image,
    fons_quad: &FonsQuad,
    base_pos: Vec2f,
    text_color: [u8; 4],
    shadow_offset: Vec2f,
    shadow_color: [u8; 4],
) {
    // shadow
    let q = quads.next_quad_mut(img);
    self::set_text_quad(q, fons_quad, base_pos + shadow_offset, shadow_color);
    // text
    let q = quads.next_quad_mut(img);
    self::set_text_quad(q, fons_quad, base_pos, text_color);
}
