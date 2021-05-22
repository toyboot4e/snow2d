/*!
UI nodes (renderables)

[`Handle`] of UI nodes are strong references, so a node won't freed until nothing refers to it.

[`Handle`]: crate::utils::pool::Handle

# `Into` impls

`Draw` variants -> `Draw` -> `Node`
*/

// Re-exported as [`Node`] variants
pub use crate::gfx::tex::{NineSliceSprite, SpriteData};

use imgui::{im_str, Ui};
use rokol::fons::FontTexture;

use crate::{
    gfx::{draw::*, geom2d::*, Color},
    ui::Node,
    utils::Inspect,
};

/// Rendering order [0, 1] (the higher, the latter)
pub type Order = f32;

/// Common geometry data that animations can operate on
#[derive(Debug, PartialEq, Clone, Default, Inspect)]
pub struct DrawParams {
    pub pos: Vec2f,
    pub size: Vec2f,
    pub color: Color,
    /// Rotation in radian
    pub rot: Option<f32>,
    pub origin: Option<Vec2f>,
    // pub scales: Vec2f,
}

impl DrawParams {
    /// Sets up quad parameters
    pub fn setup_quad<'a, 'b: 'a, B: QuadParamsBuilder>(&self, builder: &'b mut B) -> &'a mut B {
        let b = builder
            .dst_pos_px(self.pos)
            .dst_size_px(self.size)
            .color(self.color);

        if let Some(rot) = self.rot {
            b.rot(rot);
        }

        if let Some(origin) = self.origin {
            b.origin(origin);
        }

        b
    }

    pub fn transform_mut(&self, other: &mut DrawParams) {
        other.pos += self.pos;
    }
}

/// [`Node`] surface
#[derive(Debug, Clone, PartialEq)]
pub enum Surface {
    Sprite(SpriteData),
    NineSlice(NineSliceSprite),
    Text(Text),
    /// The node is only for parenting
    None,
}

impl Inspect for Surface {
    fn inspect(&mut self, ui: &Ui, label: &str) {
        match self {
            Self::Sprite(x) => x.inspect(ui, label),
            Self::NineSlice(x) => x.inspect(ui, label),
            Self::Text(x) => x.inspect(ui, label),
            Self::None => ui.label_text(&im_str!("{}", label), &im_str!("None")),
        }
    }
}

/// SurfaceVariant -> Surface -> Node
macro_rules! impl_into_draw {
    ($ty:ident, $var:ident) => {
        impl From<$ty> for Surface {
            fn from(x: $ty) -> Surface {
                Surface::$var(x)
            }
        }

        impl From<$ty> for Node {
            fn from(x: $ty) -> Node {
                Node::from(Surface::from(x))
            }
        }

        impl From<&$ty> for Surface {
            fn from(x: &$ty) -> Surface {
                Surface::$var(x.clone())
            }
        }

        impl From<&$ty> for Node {
            fn from(x: &$ty) -> Node {
                Node::from(Surface::from(x.clone()))
            }
        }
    };
}

impl_into_draw!(SpriteData, Sprite);
impl_into_draw!(NineSliceSprite, NineSlice);
impl_into_draw!(Text, Text);

/// [`Surface`] variant
#[derive(Debug, Clone, PartialEq, Inspect)]
pub struct Text {
    pub txt: String,
    // TODO: batch these types?
    pub fontsize: f32,
    pub ln_space: f32,
    // `size` and `origin` is set in `DrawParams`
    // TODO: decoration information (spans for colors, etc)
}

#[derive(Debug, Clone)]
pub struct TextBuilder<'a> {
    /// Measure text with default or user-defined parameters
    tex: &'a FontTexture,
    text: Text,
    origin: Vec2f,
}

impl<'a> TextBuilder<'a> {
    pub fn new(txt: String, tex: &'a FontTexture) -> Self {
        Self {
            tex,
            text: Text {
                txt,
                fontsize: 20.0,
                ln_space: 4.0,
            },
            origin: Vec2f::ZERO,
        }
    }

    pub fn build(self) -> Node {
        let size =
            self.tex
                .text_size_multiline(&self.text.txt, self.text.fontsize, self.text.ln_space);

        let mut node = Node::from(self.text);
        node.params.size = Vec2f::from(size);
        node.params.origin = Some(self.origin);
        node
    }

    pub fn fontsize(&mut self, fontsize: f32) -> &mut Self {
        self.text.fontsize = fontsize;
        self
    }

    pub fn ln_space(&mut self, ln_space: f32) -> &mut Self {
        self.text.ln_space = ln_space;
        self
    }

    pub fn origin(&mut self, origin: impl Into<Vec2f>) -> &mut Self {
        self.origin = origin.into();
        self
    }

    // pub fn style(&mut self, style: FontStyle) -> &mut Self
}

impl Text {
    pub fn builder(text: String, tex: &FontTexture) -> TextBuilder {
        TextBuilder::new(text, tex)
    }
}
