/*!
[`Inspect`] implementations for external crates that `snow2d` is dependent on
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
