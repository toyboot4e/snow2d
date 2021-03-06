/*!
Retained-mode rendering

UI (scene graph) is a container of sprites and animations.
*/

// TODO: refactor with indextree-like arena on top of toy_arena
// TODO: Consider whether animation arena should be handled equally as user data
// The difference is that `ui::anim` doesn't need to refer to user data type to make changes

pub mod anim;
pub mod anim_builder;
pub mod node;

use igri::{imgui, Inspect};
use std::time::Duration;

use crate::{
    gfx::{draw::*, geom2d::Vec2f, RenderPass},
    utils::{
        arena::Arena,
        enum_dispatch, ez,
        pool::{Handle, Pool, Slot, WeakHandle},
        Cheat,
    },
};

use self::{
    anim::*,
    anim_builder::AnimSeq,
    node::{DrawParams, Order, Surface},
};

/// Handle of [`Node`] in [`NodePool`]
pub type NodeHandle = crate::utils::pool::Handle<Node>;

/// Index of [`Anim`] in [`AnimStorage`]
pub type AnimIndex = crate::utils::arena::Index<Anim>;

#[derive(Debug, Clone, Copy, PartialEq, Hash, Inspect)]
pub enum CoordSystem {
    /// Use fixed position to the screen
    Screen,
    /// Used world coordinates to render nodes. Follow camera automatically
    World,
}

/// Specifies coordinate system and z ordering
// TODO: maybe use `Rc`?
#[derive(Debug, Clone, Copy, PartialEq, Inspect)]
pub struct Layer {
    pub coord: CoordSystem,
    /// 0 to 1
    pub z_order: f32,
}

/// Visible object in a UI layer
#[derive(Debug, Clone, PartialEq, Inspect)]
pub struct Node {
    pub surface: Surface,
    /// Common geometry data
    pub params: DrawParams,
    /// Draw parameter calculated befre rendering
    pub(super) cache: DrawParams,
    /// Render layer: z ordering and coordinate system
    pub layer: Layer,
    /// Local rendering order in range [0, 1] (the higher, the latter drawn)
    pub z_order: Order,
    /// NOTE: Parents are alive if any children is alive
    #[inspect(with = "inspect_option")]
    pub(super) parent: Option<Handle<Node>>,
    pub(super) children: Vec<WeakHandle<Node>>,
    // TODO: dirty flag,
}

fn inspect_option<T: Inspect>(x: &mut Option<T>, ui: &imgui::Ui, label: &str) {
    match x {
        Some(x) => x.inspect(ui, label),
        None => ui.label_text(label, "None"),
    }
}

impl From<Surface> for Node {
    fn from(draw: Surface) -> Self {
        let params = DrawParams {
            size: match draw {
                // FIXME: parent box size. Node builder?
                Surface::None => [1.0, 1.0].into(),
                Surface::Sprite(ref x) => x.sub_tex_size_scaled().into(),
                Surface::NineSlice(ref x) => x.sub_tex_size_scaled().into(),
                // FIXME: measure text size?
                Surface::Text(ref _x) => [1.0, 1.0].into(),
            },
            ..Default::default()
        };

        Node {
            surface: draw,
            params: params.clone(),
            cache: params.clone(),
            layer: Layer {
                coord: CoordSystem::Screen,
                z_order: 0.989,
            },
            z_order: 1.0,
            children: vec![],
            parent: None,
        }
    }
}

impl Node {
    pub fn global_z_order(&self) -> f32 {
        // FIXME:
        self.layer.z_order + self.z_order / 10.0
    }

    pub fn render(&mut self, pass: &mut RenderPass<'_>) {
        let params = &self.cache;
        match self.surface {
            Surface::Sprite(ref x) => {
                params.setup_quad(&mut pass.sprite(x));
            }
            Surface::NineSlice(ref x) => {
                params.setup_quad(&mut pass.sprite(x));
            }
            Surface::Text(ref text) => {
                let origin = params.origin.unwrap_or(Vec2f::ZERO);
                let pos = params.pos + params.size * origin;

                let fb = pass.fontbook();
                fb.tex.set_font(text.font.font_ix());
                fb.tex.set_size(text.fontsize);

                pass.text(pos, &text.txt, text.fontsize, text.ln_space);
            }
            Surface::None => {}
        }
    }
}

/// One of [`AnimImpl`] impls
#[enum_dispatch(AnimImpl)]
#[derive(Debug, Clone, Inspect)]
#[inspect(no_tag)]
pub enum Anim {
    DynAnim,
    // tweens
    PosTween,
    XTween,
    YTween,
    SizeTween,
    ColorTween,
    AlphaTween,
    RotTween,
    // ParamsTween,
}

/// Used for sorting nodes
#[derive(Debug)]
struct OrderEntry {
    /// Used to retrieve target item
    slot: Slot,
    /// Used to sort entries
    order: Order,
    // coord: CoordSystem,
}

pub struct SortedNodesMut<'a> {
    nodes: &'a mut Pool<Node>,
    orders: &'a [OrderEntry],
    pos: usize,
}

impl<'a> Iterator for SortedNodesMut<'a> {
    type Item = &'a mut Node;
    fn next(&mut self) -> Option<Self::Item> {
        let slot = self.orders.get(self.pos)?.slot;
        self.pos += 1;

        let ptr = self
            .nodes
            .get_mut_by_slot(slot)
            .expect("unable to find node!") as *mut _;
        Some(unsafe { &mut *ptr })
    }
}

/// Nodes and animations
#[derive(Debug, Inspect)]
pub struct Ui {
    pub nodes: NodePool,
    pub anims: AnimStorage,
    #[inspect(skip)]
    ord_buf: Vec<OrderEntry>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            nodes: NodePool::new(),
            anims: AnimStorage::new(),
            ord_buf: Vec::with_capacity(16),
        }
    }

    pub fn update(&mut self, dt: Duration) {
        // tick and apply animations. remove finished animations
        self.anims.update(dt, &mut self.nodes);

        // FIXME: Don't invalidate items with parent. Traverse and visit leaves first
        self.nodes.sync_refcounts_and_invalidate();

        // calculate geometry
        unsafe {
            let nodes = Cheat::new(&self.nodes);
            for node in nodes.as_mut().iter_mut().filter(|n| n.parent.is_none()) {
                // update cache
                Self::update_node_rec(nodes.clone(), node, None);
            }
        }
    }

    unsafe fn update_node_rec(
        nodes: Cheat<NodePool>,
        child: &mut Node,
        parent: Option<Cheat<Node>>,
    ) {
        // load animated paramaters to cache
        child.cache = child.params.clone();

        // apply transformation to this node
        if let Some(parent) = parent {
            parent.cache.transform_mut(&mut child.cache);
        }

        // apply transformation to children
        let parent = Cheat::new(child);

        let _ = parent
            .as_mut()
            .children
            .drain_filter(|child_handle| {
                if let Some(child) = nodes.as_mut().get_mut(child_handle) {
                    Self::update_node_rec(nodes.clone(), child, Some(parent.clone()));
                    false // keep the valid child index
                } else {
                    true // drain the dangling child index
                }
            })
            .collect::<Vec<_>>();
    }

    fn sort_nodes(&mut self) {
        self.ord_buf.clear();

        for (slot, node) in self.nodes.enumerate_items() {
            self.ord_buf.push(OrderEntry {
                slot,
                order: node.global_z_order(),
                // coord: node.layer.coord,
            });
        }

        self.ord_buf.sort_by(|e1, e2| {
            e1.order
                .partial_cmp(&e2.order)
                .expect("NAN found in ordering value of node")
        });
    }

    pub fn nodes_mut_sorted<'a>(&'a mut self) -> SortedNodesMut<'a> {
        self.sort_nodes();
        SortedNodesMut {
            nodes: &mut self.nodes.pool,
            orders: &self.ord_buf,
            pos: 0,
        }
    }

    /// FIXME: It basically ignores `node.layer.coord`.
    pub fn render_range(
        &mut self,
        range: impl std::ops::RangeBounds<f32>,
        pass: &mut RenderPass<'_>,
    ) {
        // TODO: more efficient rendering
        for node in &mut self
            .nodes_mut_sorted()
            .filter(|n| range.contains(&n.global_z_order()))
        {
            node.render(pass);
        }
    }
}

/// Extended [`Pool`] for handling tree of nodes
#[derive(Debug, Inspect)]
#[inspect(in_place)]
pub struct NodePool {
    pool: Pool<Node>,
}

impl std::ops::Deref for NodePool {
    type Target = Pool<Node>;
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl std::ops::DerefMut for NodePool {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pool
    }
}

impl NodePool {
    pub fn new() -> Self {
        Self {
            pool: Pool::with_capacity(16),
        }
    }
}

/// Parenting
impl NodePool {
    pub fn attach_child(&mut self, parent_handle: &Handle<Node>, child_handle: &Handle<Node>) {
        self.pool[parent_handle]
            .children
            .push(child_handle.to_downgraded());
    }

    /// Creates child node for a parent node
    pub fn add_child(&mut self, parent_handle: &Handle<Node>, mut child: Node) -> Handle<Node> {
        child.parent = Some(parent_handle.clone());
        let child_handle = self.pool.add(child);
        self.attach_child(parent_handle, &child_handle);
        child_handle
    }

    /// Creates parent node from child nodes. Node that child nodes are NOT owned by the parent; you
    /// have to preserve them yourself!
    pub fn add_parent(
        &mut self,
        parent: Node,
        children: Vec<Node>,
    ) -> (Handle<Node>, Vec<Handle<Node>>) {
        let parent = self.add(parent);

        let children = children
            .into_iter()
            .map(|c| self.add_child(&parent, c))
            .collect::<Vec<_>>();

        (parent, children)
    }

    /// Creates parent node from child nodes. Node that child nodes are NOT owned by the parent; you
    /// have to preserve them yourself!
    pub fn add_container(&mut self, children: Vec<Node>) -> (Handle<Node>, Vec<Handle<Node>>) {
        let container = Node::from(Surface::None);
        self.add_parent(container, children)
    }
}

#[derive(Debug, Clone, Inspect)]
pub(crate) struct DelayedAnim {
    delay: ez::LinearDt,
    is_first_tick: bool,
    #[inspect(skip)]
    anim: Anim,
}

impl DelayedAnim {
    pub fn new(delay: Duration, anim: Anim) -> Self {
        Self {
            delay: ez::LinearDt::new(delay.as_secs_f32()),
            is_first_tick: false,
            anim,
        }
    }
}

/// Extended [`Arena`] for animations
///
/// TODO: guarantee no duplicates exist
#[derive(Debug)]
pub struct AnimStorage {
    running: Arena<Anim>,
    delayed: Vec<DelayedAnim>,
}

impl Inspect for AnimStorage {
    fn inspect(&mut self, ui: &imgui::Ui, label: &str) {
        igri::nest(ui, label, || {
            igri::nest(ui, "running", || {
                for (i, (_index, x)) in self.running.iter_mut().enumerate() {
                    Inspect::inspect(x, ui, &format!("{}", i));
                }
            });

            igri::seq(self.delayed.iter_mut().map(|x| x), ui, "delayed");
        });
    }
}

impl AnimStorage {
    pub fn new() -> Self {
        Self {
            running: Arena::with_capacity(16),
            delayed: Vec::with_capacity(16),
        }
    }

    pub fn insert_seq(&mut self, seq: AnimSeq) {
        for anim in seq.anims {
            self.delayed.push(anim);
        }
    }

    pub fn insert_delayed(&mut self, delay: Duration, anim: Anim) {
        self.delayed.push(DelayedAnim::new(delay, anim));
    }
}

impl std::ops::Deref for AnimStorage {
    type Target = Arena<Anim>;
    fn deref(&self) -> &Self::Target {
        &self.running
    }
}

impl std::ops::DerefMut for AnimStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.running
    }
}

impl AnimStorage {
    /// Tick and apply animations. Remove finished animations
    pub fn update(&mut self, dt: Duration, nodes: &mut Pool<Node>) {
        // update `delayed` animations
        let new_start_anims = self.delayed.drain_filter(|anim| {
            // TODO: refactor with Timer, maybe in `ez`
            if anim.is_first_tick {
                anim.is_first_tick = false;

                // first tick: do end check BEFORE ticking
                if anim.delay.is_end() {
                    true // drain
                } else {
                    anim.delay.tick(dt);
                    false
                }
            } else {
                // non-first tick: do end check AFTER ticking
                anim.delay.tick(dt);
                anim.delay.is_end()
            }
        });

        for mut anim in new_start_anims.map(|delayed| delayed.anim) {
            anim.set_active(true);
            self.running.insert(anim);
        }

        // update `running` animations
        let mut drain = Vec::new();
        for mut entry in self.running.bindings() {
            let anim = entry.get_mut();
            if !anim.is_active() {
                continue;
            }
            if anim.is_end() {
                let anim = entry.remove();
                drain.push(anim);
                continue;
            }
            anim.tick(dt);
            anim.apply(nodes);
        }
    }
}
