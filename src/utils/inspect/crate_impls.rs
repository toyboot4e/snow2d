/*!
[`Inspect`] implementations

My crates optionally depend on `igri` and derives `Inspect`, so there's not so much work left.
*/

use igri::{imgui, Inspect};
use imgui::Ui;

use crate::{asset::Asset, gfx::tex::*};

// TODO: prefer blanket impl
impl Inspect for Asset<Texture2dDrop> {
    fn inspect(&mut self, ui: &Ui, _label: &str) {
        ui.text(format!("Asset at {}", self.path().display()));
    }
}
