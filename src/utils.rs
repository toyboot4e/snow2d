/*!
Core utilities and handy tools

[`arena`], [`ez`], [`Inspect`], [`pool`] and [`tyobj`] are the core utilities. Other are just handy
tools.
*/

mod cheat;
pub use cheat::{cheat, Cheat};

pub mod inspect;
pub use inspect::Inspect;

#[doc(inline)]
pub use toy_arena as arena;

#[doc(inline)]
pub use toy_pool as pool;

pub mod ez;
pub mod smpsc;

pub mod tyobj;

pub mod tweak {
    /*!
    Re-expoted from [inline_tweak]

    Create tweakable literal at runtime on debug build:

    ```
    use snow2d::utils::tweak::*;

    pub fn volume() -> f32 {
        tweak!(1.0)
    }
    ```
    */

    // `inline_tweak` has to be in scope to use `tweak!`
    pub use inline_tweak::{self, watch, Tweakable};

    /// Creates reloadable literal at runtime
    pub use inline_tweak::tweak;
}

pub use bytemuck;
pub use dyn_clone;
pub use once_cell;

// ----------------------------------------
// macros

/// Re-exported from [`arraytools`]
///
///
pub use arraytools::ArrayTools;

/// Re-exported from [`bitflags`](::bitflags)
///
///
pub use bitflags::bitflags;

/// Re-exported from [`derivative`]
///
/// ---
pub use derivative::Derivative;

/// Re-exported from `enum_dispatch`
pub use enum_dispatch::enum_dispatch;

/// Re-exported from `hackfn`
pub use hackfn::hackfn;

/// Re-exported from `indoc`
pub use indoc::indoc;

/// Re-exported from `inherent`
pub use inherent::inherent;
