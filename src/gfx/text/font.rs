/*!
Font data descripting types
*/

use std::path::PathBuf;

pub use rokol::fons::{fontstash::FontIx, FontTexture};

// --------------------------------------------------------------------------------
// Desc types for loading fonts

/// Bytes loading description
#[derive(Debug)]
pub enum LoadDesc<'a> {
    Path(PathBuf),
    Mem(&'a [u8]),
}

impl<'a> From<PathBuf> for LoadDesc<'a> {
    fn from(x: PathBuf) -> Self {
        Self::Path(x)
    }
}

impl<'a> From<&'a [u8]> for LoadDesc<'a> {
    fn from(x: &'a [u8]) -> Self {
        Self::Mem(x)
    }
}

#[derive(Debug)]
pub struct FontSetDesc<'a> {
    pub name: String,
    pub regular: FontDesc<'a>,
    pub bold: Option<FontDesc<'a>>,
    pub italic: Option<FontDesc<'a>>,
}

#[derive(Debug)]
pub struct FontDesc<'a> {
    pub name: String,
    pub load: LoadDesc<'a>,
}
