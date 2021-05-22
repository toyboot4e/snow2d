/*!
`snow2d` ❄️ framework

It's a 2D framework specific for my roguelike game. **It doesn't meet others' needs**.
*/

#![feature(drain_filter)]

pub extern crate rokol;

pub mod asset;
pub mod audio;
pub mod gfx;
pub mod input;

pub mod prelude;
pub mod ui;
pub mod utils;

use std::time::{Duration, Instant};

use rokol::gfx as rg;
use sdl2::event::{Event, WindowEvent};

use crate::{
    asset::AssetCacheAny,
    audio::asset::MusicPlayer,
    audio::Audio,
    gfx::{Color, Snow2d, WindowState},
    input::Input,
};

/// Generic game context
#[derive(Debug)]
pub struct Ice {
    /// Clears target (frame buffer by default) with cornflower blue color
    pa_blue: rg::PassAction,
    /// 2D renderer
    pub snow: Snow2d,
    /// Audio context
    pub audio: Audio,
    /// Background music player
    pub music_player: MusicPlayer,
    /// Asset cache for any type
    pub assets: AssetCacheAny,
    pub input: Input,
    /// Delta time from last frame
    dt: Duration,
    frame_count: u64,
}

impl Ice {
    pub fn new(snow: Snow2d) -> Self {
        // TODO: don't unwrap
        let audio = unsafe { Audio::create().unwrap() };

        Self {
            pa_blue: rg::PassAction::clear(Color::CORNFLOWER_BLUE.to_normalized_array()),
            snow,
            audio: audio.clone(),
            music_player: MusicPlayer::new(audio.clone()),
            assets: AssetCacheAny::new(),
            input: Input::new(),
            dt: Duration::new(0, 0),
            frame_count: 0,
        }
    }

    pub fn dt(&self) -> Duration {
        self.dt
    }

    /// How many times did we call [`pre_update`](Self::pre_update)
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

/// Lifecycle
impl Ice {
    /// Updates input state
    pub fn event(&mut self, ev: &sdl2::event::Event) {
        self.input.event(ev);
    }

    /// Updates frame counter
    pub fn pre_update(&mut self, dt: Duration) {
        self.frame_count += 1;
        self.dt = dt;
        self.snow.clock.tick(dt);
    }

    /// Updates font texture
    pub fn pre_render(&mut self, window: WindowState) {
        self.snow.pre_render(window);
    }

    pub fn post_render(&mut self, dt: Duration) {
        self.snow.post_render(dt);
    }

    /// TODO: Debug render?
    pub fn render(&mut self) {
        // debug render?
    }

    /// Updates asset reference counts  and swaps input data buffers
    pub fn on_end_frame(&mut self) {
        self.assets.free_unused();
        self.input.on_end_frame();
    }
}

/// Utility for updating the game at 60 FPS durling the window has focus
#[derive(Debug)]
pub struct GameRunner {
    target_dt: Duration,
    now: Instant,
    accum: Duration,
    focus: [bool; 2],
}

impl GameRunner {
    pub fn new() -> Self {
        Self {
            target_dt: Duration::from_nanos(1_000_000_000 / 60),
            now: Instant::now(),
            accum: Duration::default(),
            focus: [false, false],
        }
    }

    pub fn dt(&self) -> Duration {
        self.target_dt
    }
}

/// Lifecycle
impl GameRunner {
    #[inline(always)]
    pub fn event(&mut self, ev: &Event) {
        match ev {
            Event::Window {
                // main `window_id` is `1`
                // window_id,
                win_event,
                ..
            } => match win_event {
                // keyborad focus
                WindowEvent::FocusLost => {
                    // log::trace!("focus lost: {:?}", window_id);
                    self.focus[1] = false;
                }
                WindowEvent::FocusGained => {
                    // log::trace!("gain: {:?}", window_id);
                    self.focus[1] = true;
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Returns true if the game should be updated this frame
    #[inline(always)]
    pub fn update(&mut self) -> bool {
        let update = match (self.focus[0], self.focus[1]) {
            (false, true) => {
                // gain focus
                self.accum = Duration::default();
                self.now = Instant::now();

                false
            }
            (true, false) => {
                // lose focus
                self.accum = Duration::default();
                self.now = Instant::now();

                false
            }
            (true, true) => {
                // been focused
                let new_now = Instant::now();
                self.accum += new_now - self.now;
                self.now = new_now;

                true
            }
            (false, false) => {
                // been unfocused: stop the game
                false
            }
        };
        self.focus[0] = self.focus[1];
        update
    }
}
