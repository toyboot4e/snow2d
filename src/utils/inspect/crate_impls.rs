/*!
[`Inspect`] implementations for external crates that `snow2d` is dependent on
*/

use imgui::{im_str, Ui};

use crate::{
    asset::Asset,
    gfx::tex::*,
    input::Dir8,
    utils::{arena, pool},
};

use super::Inspect;

impl Inspect for Dir8 {
    fn inspect(&mut self, ui: &Ui, _label: &str) {
        ui.text("TODO: Dir8");
    }
}

impl<T: Inspect> Inspect for arena::Arena<T> {
    fn inspect(&mut self, ui: &Ui, label: &str) {
        imgui::TreeNode::new(&imgui::im_str!("{}", label))
            .flags(imgui::TreeNodeFlags::OPEN_ON_ARROW | imgui::TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
            .build(ui, || {
                for (i, item) in self.items_mut().enumerate() {
                    item.inspect(ui, im_str!("{}", i).to_str());
                }
            });
    }
}

impl<T> Inspect for arena::Index<T> {
    fn inspect(&mut self, ui: &Ui, _label: &str) {
        ui.text("TODO: Index<T>");
    }
}

impl Inspect for Asset<Texture2dDrop> {
    fn inspect(&mut self, ui: &Ui, _label: &str) {
        ui.text(format!("Asset at {}", self.path().display()));
    }
}

impl<T> Inspect for pool::Handle<T> {
    fn inspect(&mut self, ui: &Ui, _label: &str) {
        ui.text("TODO: Handle<T>");
    }
}

impl<T> Inspect for pool::WeakHandle<T> {
    fn inspect(&mut self, ui: &Ui, _label: &str) {
        ui.text("TODO: WeakHandle<T>");
    }
}

impl<T: Inspect + 'static> Inspect for pool::Pool<T> {
    fn inspect(&mut self, ui: &Ui, label: &str) {
        imgui::TreeNode::new(&imgui::im_str!("{}", label))
            .flags(imgui::TreeNodeFlags::OPEN_ON_ARROW | imgui::TreeNodeFlags::OPEN_ON_DOUBLE_CLICK)
            .build(ui, || {
                for (i, item) in self.iter_mut().enumerate() {
                    item.inspect(ui, im_str!("{}", i).to_str());
                }
            });
    }
}
