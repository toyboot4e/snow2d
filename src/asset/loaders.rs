/*!
Builtin asset loaders
*/

use std::{fmt, io};

use crate::{
    asset::{self, AssetCache, AssetItem, AssetLoader},
    audio::{src, Audio},
    gfx::tex::{Texture2dDrop, TextureBuilder},
};

pub(crate) fn register(assets: &mut AssetCache, audio: Audio) {
    assets.add_cache::<Texture2dDrop>(TextureLoader);
    add_src::<src::Wav>(assets, audio.clone());
    add_src::<src::WavStream>(assets, audio.clone());

    fn add_src<T>(assets: &mut AssetCache, audio: Audio)
    where
        T: crate::audio::prelude::FromExt + fmt::Debug + 'static,
    {
        assets.add_cache::<T>(AudioLoader {
            audio,
            _ty: std::marker::PhantomData,
        });
    }
}

// --------------------------------------------------------------------------------
// Texture support

/// [`AssetLoader`] for [`Texture2dDrop`]
#[derive(Debug)]
pub struct TextureLoader;

impl AssetItem for Texture2dDrop {
    type Loader = TextureLoader;
}

impl AssetLoader for TextureLoader {
    type Item = Texture2dDrop;

    fn load(&self, bytes: Vec<u8>, _context: &mut AssetCache) -> asset::Result<Self::Item> {
        use std::io::{Error, ErrorKind};

        let tex = TextureBuilder::from_encoded_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))?
            .build_texture();

        Ok(tex)
    }
}

// --------------------------------------------------------------------------------
// Audio support

/// [`AssetLoader`] for audio source types
#[derive(Debug)]
pub struct AudioLoader<Src>
where
    Src: crate::audio::prelude::FromExt + fmt::Debug + 'static,
{
    audio: Audio,
    _ty: std::marker::PhantomData<Src>,
}

impl<T> AssetItem for T
where
    T: crate::audio::prelude::FromExt + fmt::Debug + 'static,
{
    type Loader = AudioLoader<T>;
}

impl<T> AssetLoader for AudioLoader<T>
where
    T: crate::audio::prelude::FromExt + fmt::Debug + 'static,
{
    type Item = T;
    fn load(&self, bytes: Vec<u8>, _context: &mut AssetCache) -> io::Result<Self::Item> {
        Self::Item::from_mem(bytes).map_err(self::upcast_err)
    }
}

fn upcast_err<E>(e: E) -> std::io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    std::io::Error::new(std::io::ErrorKind::Other, e)
}
