/*!
Audio suport

That is, [`soloud-rs`] re-exported with additional types and [`asset`](../asset) integration.

[`soloud-rs`]: https://github.com/MoAlyousef/soloud-rs
---

[SoLoud] is an easy to use, free, portable c/c++ audio engine for games.

[SoLoud]: https://sol.gfxile.net/soloud/
*/

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use anyhow::*;

pub use soloud::{audio as src, filter, prelude, Handle, Soloud as AudioDrop};

use crate::asset::{Asset, AssetCache, AssetKey};

/// Shared audio context
#[derive(Debug, Clone)]
pub struct Audio {
    /// NOTE: We can't use `RefCell` if we want to implement `Deref` for `Audio`. We can d with
    /// unsafe cell, but it requires us ensure we never break the aliasing rule.
    inner: Rc<UnsafeCell<AudioDrop>>,
}

impl Audio {
    /// Make sure to not create [`Audio`] twice
    pub unsafe fn create() -> Result<Self, prelude::SoloudError> {
        let inner = AudioDrop::default()?;
        Ok(Self {
            inner: Rc::new(UnsafeCell::new(inner)),
        })
    }
}

// cheat the borrow checker..

/// # Safety
/// It's safe because every function of `AudioDrop` is one-shot and releases `self` immediately
/// after the call
impl Deref for Audio {
    type Target = AudioDrop;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.get() }
    }
}

/// # Safety
/// It's safe because every function of `AudioDrop` is one-shot and releases `self` immediately
/// after the call
impl DerefMut for Audio {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // unsafe { &mut *(Rc::as_ptr(&self.inner) as *mut _) }
        unsafe { &mut *self.inner.get() }
    }
}

// --------------------------------------------------------------------------------
// Extensions

/// Playback handle for [`MusicPlayer`]
#[derive(Debug)]
pub struct Playback {
    pub handle: Handle,
    pub song: Asset<src::WavStream>,
}

/// Background music player
#[derive(Debug)]
pub struct MusicPlayer {
    pub audio: Audio,
    /// [`Playback`] of current music
    pub current: Option<Playback>,
}

impl MusicPlayer {
    pub fn new(audio: Audio) -> Self {
        Self {
            audio,
            current: None,
        }
    }

    pub fn play_song(&mut self, mut song: Asset<src::WavStream>) {
        if let Some(_playback) = self.current.as_mut() {
            // TODO: fade out
        }

        // TODO: fade in
        let handle =
            self.audio
                .play_background_ex(&*song.get_mut().unwrap(), 1.0, false, Handle::PRIMARY);

        self.current = Some(Playback { handle, song })
    }
}

impl AssetCache {
    /// Play sound
    pub fn play<'a>(&mut self, sound: impl Into<AssetKey<'a>>, audio: &Audio) -> Result<()> {
        let mut se: Asset<src::Wav> = self.load_sync(sound)?;
        let se = se.get_mut().unwrap();
        audio.play(&*se);
        Ok(())
    }

    /// Play sound and set the preserve flag on the asset
    pub fn play_preserve<'a>(
        &mut self,
        sound: impl Into<AssetKey<'a>>,
        audio: &Audio,
    ) -> Result<()> {
        let mut se: Asset<src::Wav> = self.load_sync_preserve(sound)?;
        let se = se.get_mut().unwrap();
        audio.play(&*se);
        Ok(())
    }
}
