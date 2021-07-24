/*!
Input support

That is, [`xdl`] re-exported. Currently, virtual key is oriented for orthogonal grid maps.

TODO: gamepad and mouse
*/

pub use xdl::{utils, Dir4, Dir8, Input, Key, Keyboard, Sign};

pub mod vi {
    //! Virtual input

    pub use snow2d_macros::keys;
    pub use xdl::vi::*;
}
