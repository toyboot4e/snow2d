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
    asset::AssetCache,
    audio::asset::MusicPlayer,
    audio::Audio,
    gfx::{Color, Snow2d, WindowState},
    input::Input,
};

/// Generic game context
///
/// Bundle of graphics, input, audio, assets and utilities.
#[derive(Debug)]
pub struct Ice {
    /// Clears target (frame buffer by default) with cornflower blue color
    pa_blue: rg::PassAction,
    /// 2D renderer
    pub snow: Snow2d,
    /// All the input states
    pub input: Input,
    /// Audio context
    pub audio: Audio,
    /// Asset cache for any type
    pub assets: AssetCache,
    /// Background music player
    pub music: MusicPlayer,
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
            music: MusicPlayer::new(audio.clone()),
            assets: AssetCache::new(),
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

/// Simple FPS watchcer
#[derive(Debug, Clone, Default)]
pub struct Fps {
    /// Average FPS per frame in seconds
    avg: f64,
    /// Square of spike FPS in seconds
    spike: f64,
}

impl Fps {
    const K: f64 = 0.05;

    /// Update FPS counts with real time
    pub fn update(&mut self, frame_time: Duration) {
        self.update_avg(frame_time.as_secs_f64());
        self.update_spike(frame_time.as_secs_f64());
    }

    /// Average FPS
    pub fn avg(&self) -> f64 {
        1.0 / self.avg
    }

    /// Spike FPS
    pub fn spike(&self) -> f64 {
        self.spike.sqrt()
    }

    fn update_avg(&mut self, dt: f64) {
        self.avg *= 1.0 - Self::K;
        self.avg += dt * Self::K;
    }

    fn update_spike(&mut self, dt: f64) {
        self.spike *= 1.0 - Self::K;
        self.spike += (dt * dt) * Self::K;
    }
}

/// Run your game at 60 FPS (trait-free)
///
/// TODO: Error handling
///
/// Details are in the source code.
#[inline(always)]
pub fn run<S>(
    mut pump: sdl2::EventPump,
    state: &mut S,
    mut event: impl FnMut(&mut S, &Event),
    mut frame: impl FnMut(&mut S, Duration),
) {
    let mut runner = self::GameRunner::new();

    'game_loop: loop {
        for ev in pump.poll_iter() {
            let poll_start = Instant::now();

            if matches!(ev, sdl2::event::Event::Quit { .. }) {
                break 'game_loop;
            }

            runner.event(&ev);
            (event)(state, &ev);

            let dt = Instant::now() - poll_start;
            log::trace!("poll: {:?} | {:?}", dt, ev);
        }

        let tick = runner.update();

        if !tick {
            // not focused
            std::thread::sleep(std::time::Duration::from_secs_f32(0.2));
            continue;
        }

        if let Some(dt) = runner.timestep() {
            // focused & update
            (frame)(state, dt);
        } else {
            // focusd & iding
            if let Some(dt) = runner.sleep_duration() {
                std::thread::sleep(dt);
            } else {
                eprintln!("AAAAAAAAAAAAAAAAAAAAAAAAA??");
            }
        }
    }
}

/// Utility for updating the game at 60 FPS while the window has focus
#[derive(Debug)]
struct GameRunner {
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
            accum: Duration::ZERO,
            focus: [false, false],
        }
    }
}

impl GameRunner {
    /// Watch window focus state
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
}

impl GameRunner {
    /// Updates the accumulated duration
    #[inline(always)]
    pub fn update(&mut self) -> bool {
        let tick = self.update_focus();

        if tick {
            let new_now = Instant::now();
            self.accum += new_now - self.now;
            self.now = new_now;
        } else {
            self.accum = Duration::ZERO;
            self.now = Instant::now();
        }

        tick
    }

    /// Consumes the accumulated duration and maybe creates a timestep
    #[inline(always)]
    pub fn timestep(&mut self) -> Option<Duration> {
        // Consume the accumulated duration.
        // It may correspond to multiple timesteps.
        let mut n_steps = 0;
        while self.consume() {
            n_steps += 1;
        }

        if n_steps > 0 {
            // Update only once, but with a `dt` propertional to the number of steps
            Some(self.target_dt * n_steps)
        } else {
            None
        }
    }

    /// Sleep duration for idling
    pub fn sleep_duration(&self) -> Option<Duration> {
        if self.accum >= Duration::from_secs_f64(1.0 / 61.0) {
            None
        } else {
            Some(self.target_dt - self.accum)
        }
    }

    #[inline(always)]
    fn update_focus(&mut self) -> bool {
        let tick = match (self.focus[0], self.focus[1]) {
            (false, true) => {
                // on gain focus
                false
            }
            (true, false) => {
                // on lose focus
                false
            }
            (true, true) => {
                // been focused
                true
            }
            (false, false) => {
                // been unfocused
                false
            }
        };

        self.focus[0] = self.focus[1];
        tick
    }

    /**
    https://medium.com/@tglaiel/how-to-make-your-game-run-at-60fps-24c61210fe75

    ```c++
    while(accumulator >= 1.0/61.0){
        simulate_update();
        accumulator -= 1.0/60.0;
        if(accumulator < 1.0/59.0–1.0/60.0) accumulator = 0;
    }
    ```
    */
    fn consume(&mut self) -> bool {
        if self.accum >= Duration::from_secs_f64(1.0 / 61.0) {
            if self.accum < Duration::from_secs_f64(1.0 / 59.0) {
                self.accum = Duration::ZERO;
            } else {
                self.accum -= Duration::from_secs_f64(1.0 / 60.0);
            }
            true
        } else {
            false
        }
    }
}
