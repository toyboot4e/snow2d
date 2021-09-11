//! Builtin shaders

#![allow(unused)]

use rokol::gfx::{self as rg, BakedResource, VertexLayout};

use rokol::ffi::gfx as rfg;

use crate::{
    gfx::Shader,
    utils::bytemuck::{Pod, Zeroable},
};

/// Creates a null-terminated string from static string
macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0")
    };
}

/// Embed shader files (`[&str; 2]`)
macro_rules! def_shd {
    ($file:expr) => {
        [
            concat!(include_str!(concat!("shaders/glsl/", $file, ".vs")), "\0").to_string(),
            concat!(include_str!(concat!("shaders/glsl/", $file, ".fs")), "\0").to_string(),
        ]
    };
}

macro_rules! img_type {
    ($name:expr,$ty:expr) => {
        rg::ShaderImageDesc {
            name: c_str!($name).as_ptr() as *const _,
            image_type: $ty.to_ffi(),
            ..Default::default()
        }
    };
}

/// Single-value uniform block
macro_rules! ub {
    ($name:expr, $uniform_ty:expr, $size_ty:ty) => {{
        let mut block = rg::ShaderUniformBlockDesc::default();

        block.uniforms[0] = rg::ShaderUniformDesc {
            name: concat!($name, "\0").as_ptr() as *const _,
            type_: $uniform_ty.to_ffi(),
            ..Default::default()
        };
        block.size += std::mem::size_of::<$size_ty>() as u64;

        block
    }};
}

/// Creates vertex & fragment shader from files and modifies them with given procedure
fn gen(
    vs_fs: &[impl AsRef<str>; 2],
    mut_shd_desc: impl Fn(&mut rg::ShaderDesc),
    pip_desc: &mut rg::PipelineDesc,
) -> Shader {
    let mut shd_desc = unsafe { rokol::gfx::shader_desc(vs_fs[0].as_ref(), vs_fs[1].as_ref()) };
    mut_shd_desc(&mut shd_desc);
    let shd = rg::Shader::create(&shd_desc);

    pip_desc.shader = shd;
    let pip = rg::Pipeline::create(&pip_desc);

    Shader::new(shd, pip)
}

/// Position, color and uv
#[derive(Debug, Clone, Copy, Default, VertexLayout)]
#[repr(C)]
pub struct DefaultVertex {
    pub pos: [f32; 2],
    pub color: [u8; 4],
    pub uv: [f32; 2],
}

unsafe impl Zeroable for DefaultVertex {}
unsafe impl Pod for DefaultVertex {}

impl<Pos, Color, Uv> From<(Pos, Color, Uv)> for DefaultVertex
where
    Pos: Into<[f32; 2]>,
    Color: Into<[u8; 4]>,
    Uv: Into<[f32; 2]>,
{
    fn from(data: (Pos, Color, Uv)) -> Self {
        Self {
            pos: data.0.into(),
            color: data.1.into(),
            uv: data.2.into(),
        }
    }
}

const ALPHA_BLEND: rg::BlendState = rg::BlendState {
    enabled: true,
    src_factor_rgb: rfg::sg_blend_factor::SG_BLENDFACTOR_SRC_ALPHA,
    dst_factor_rgb: rfg::sg_blend_factor::SG_BLENDFACTOR_ONE_MINUS_SRC_ALPHA,
    op_rgb: rfg::sg_blend_op::_SG_BLENDOP_DEFAULT,
    src_factor_alpha: rfg::sg_blend_factor::SG_BLENDFACTOR_ONE,
    dst_factor_alpha: rfg::sg_blend_factor::SG_BLENDFACTOR_ZERO,
    op_alpha: rfg::sg_blend_op::_SG_BLENDOP_DEFAULT,
};

pub fn default_screen() -> Shader {
    gen(
        &def_shd!("tex_1"),
        |desc| {
            desc.fs.images[0] = img_type!("tex1", rg::ImageType::Dim2);
            desc.vs.uniform_blocks[0] = ub!("transform", rg::UniformType::Mat4, glam::Mat4);
        },
        &mut {
            let mut desc = rg::PipelineDesc {
                index_type: rg::IndexType::UInt16.to_ffi(),
                layout: DefaultVertex::layout_desc(),
                cull_mode: rg::CullMode::None.to_ffi(),
                ..Default::default()
            };
            desc.colors[0].blend = ALPHA_BLEND;
            desc
        },
    )
}

pub fn default_offscreen() -> Shader {
    gen(
        &def_shd!("tex_1"),
        |desc| {
            desc.fs.images[0] = img_type!("tex1", rg::ImageType::Dim2);
            desc.vs.uniform_blocks[0] = ub!("transform", rg::UniformType::Mat4, glam::Mat4);
        },
        &mut {
            let mut desc = rg::PipelineDesc {
                index_type: rg::IndexType::UInt16.to_ffi(),
                layout: DefaultVertex::layout_desc(),
                cull_mode: rg::CullMode::None.to_ffi(),
                ..Default::default()
            };
            desc.depth.pixel_format = rg::PixelFormat::Depth.to_ffi();
            desc
        },
    )
}

/// Tow-pass gaussian blur for ping pong frame buffers
///
/// <https://learnopengl.com/Advanced-Lighting/Bloom>
pub fn gauss() -> Shader {
    gen(
        &def_shd!("gauss"),
        |desc| {
            desc.fs.images[0] = img_type!("tex1", rg::ImageType::Dim2);
            desc.vs.uniform_blocks[0] = ub!("transform", rg::UniformType::Mat4, glam::Mat4);
            desc.vs.uniform_blocks[1] = ub!("is_horizontal", rg::UniformType::Float, f32);
        },
        &mut {
            let mut desc = rg::PipelineDesc {
                index_type: rg::IndexType::UInt16.to_ffi(),
                layout: DefaultVertex::layout_desc(),
                cull_mode: rg::CullMode::None.to_ffi(),
                ..Default::default()
            };
            desc.depth.pixel_format = rg::PixelFormat::Depth.to_ffi();
            desc
        },
    )
}

#[derive(Debug, Clone, Copy, Default, VertexLayout)]
#[repr(C)]
pub struct PosUvVert {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
}

unsafe impl Zeroable for PosUvVert {}
unsafe impl Pod for PosUvVert {}

pub fn snow() -> Shader {
    gen(
        &def_shd!("snow"),
        |desc| {
            desc.fs.uniform_blocks[0] = ub!("iResolution", rg::UniformType::Float2, [f32; 2]);
            desc.fs.uniform_blocks[1] = ub!("iTime", rg::UniformType::Float, f32);
            desc.fs.uniform_blocks[2] = ub!("iMouse", rg::UniformType::Float2, [f32; 2]);
        },
        &mut {
            let mut desc = rg::PipelineDesc {
                index_type: rg::IndexType::UInt16.to_ffi(),
                layout: PosUvVert::layout_desc(),
                cull_mode: rg::CullMode::None.to_ffi(),
                ..Default::default()
            };
            desc.colors[0].blend = ALPHA_BLEND;
            desc
        },
    )
}
