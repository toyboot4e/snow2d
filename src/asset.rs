/*!
Asset management

[`Asset<T>`] is a shared reference of an asset item. Any type of `Asset` is stored in
[`AssetCache`].

# Asset path

Asset directory is assumed to be at `manifest_dir/assets`. [`AssetKey`] is either of the two:

* "relative/path/to/the/asset/directory"
* "scheme:relative/path/to/the/scheme/directory"

TODO: `<asset_dir>/schemes.txt`

# Serde support

Every [`Asset`] is serialized as [`PathBuf`] and deserialiezd as [`Asset`].

WARNING: Deserialization has to be done in a [`guarded`] scope.

Reason: [`Asset`] is a shared pointer and we need to take care to not create duplicates. But `serde`
doesn't let us share states while deserialization. So we need a thread-local pointer, which is only
valid in the [`with_cache`] procedure.

# Context of asset loaders

There are basically two ways to give context to asset loaders:

1. Contexts are shared among asset loaders and other types
2. Contexts are given to the loader from external
  2-1. As a concrete, user-defined type
  2-2. As a kind of `AnyMap` (with our without automatic query)

# TODOs

* async loading
* hot reloading (tiled map, actor image, etc.)
* force `/` as path separateor

# Problems

* `Asset` implements `Deref`, but they don't implement traits that the underlying data implements
*/

// TODO: add entry type id
// TODO: remove asset size
// TODO: improve path interning
// TODO: special asset key repr for Asset<T>

#![allow(dead_code)]

pub type Error = std::io::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

use std::{
    any::{self, TypeId},
    borrow::Cow,
    cell::UnsafeCell,
    collections::HashMap,
    fmt, fs, io, mem,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    rc::Rc,
    str,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use downcast_rs::{impl_downcast, Downcast};
use once_cell::sync::OnceCell;
use serde::{
    de::{self, Deserializer},
    ser::Serializer,
    Deserialize, Serialize,
};

/// Generational index or identity of assets
type Gen = u32;

/// Data that can be used as an asset
pub trait AssetItem: fmt::Debug + Sized + 'static {
    type Loader: AssetLoader<Item = Self>;
}

/// How to load an [`AssetItem`]
pub trait AssetLoader: fmt::Debug + Sized + 'static {
    type Item: AssetItem;
    fn load(&self, path: &Path, cache: &mut AssetCache) -> Result<Self::Item>;
}

/// Mutable access to multiple asset loaders at runtime
#[derive(Debug)]
pub struct AssetCacheCell<'a> {
    cache: &'a mut AssetCache,
    /// borrow checker for loaders at runtime
    checker: Vec<(TypeId, Borrow)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Borrow {
    //
}

/// Shared ownership of an asset item. Be sure to call [`with_cache`] on deserialization.
#[derive(Debug)]
pub struct Asset<T: AssetItem> {
    item: Option<Arc<Mutex<T>>>,
    preserved: Arc<Mutex<bool>>,
    /// TODO: Load path for hot reloading
    load_path: Rc<PathBuf>,
    /// Owned asset key for serde
    serde_repr: Rc<AssetKey<'static>>,
    identity: Gen,
}

impl<T: AssetItem> Clone for Asset<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.as_ref().map(|x| Arc::clone(x)),
            preserved: Arc::clone(&self.preserved),
            load_path: Rc::clone(&self.load_path),
            serde_repr: Rc::clone(&self.serde_repr),
            identity: self.identity,
        }
    }
}

impl<T: AssetItem> std::cmp::PartialEq for Asset<T> {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}

impl<T: AssetItem> Asset<T> {
    pub fn path(&self) -> &Path {
        self.load_path.as_ref()
    }

    pub fn is_loaded(&self) -> bool {
        self.item.is_some()
    }

    pub fn set_preserved(&mut self, b: bool) {
        *self.preserved.lock().unwrap() = b;
    }

    /**
    Tries to get `&T`, fails if the asset is not loaded or failed to load

    This step is for asynchrounous loading and hot reloaidng.

    Unfortunatelly, the return type is not `Option<&T>` and doesn't implement trait for type `T`.
    Still, you can use `&*asset.get()` to cast it to `&T`.
    */
    pub fn get<'a>(&'a self) -> Option<impl Deref<Target = T> + 'a> {
        self.item.as_ref()?.lock().ok()
    }

    /**
    Tries to get `&mut T`, fails if the asset is not loaded or panics ([`Mutex`] under the hood)

    This step is for asynchrounous loading and hot reloaidng.

    Unfortunatelly, the return type is not `Option<&mut T>` and doesn't implement trait for type
    `T`. Still, you can use `&mut *asset.get_mut()` to cast it to `&mut T`.
    */
    pub fn get_mut<'a>(&'a mut self) -> Option<impl DerefMut<Target = T> + 'a> {
        self.item.as_mut()?.lock().ok()
    }
}

/// URI for an asset resolved with [`AssetCache::resolve`]. It's a relative path from either asset
/// directory or a directory specified by scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetKey<'a> {
    scheme: Option<Cow<'a, Path>>,
    path: Cow<'a, Path>,
}

impl<'a, 'de> Deserialize<'de> for AssetKey<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let x = String::deserialize(deserializer)?;
        let key: Self = x.parse().map_err(de::Error::custom)?;
        // TODO: validate the key?
        Ok(key)
    }
}

impl<'a> Serialize for AssetKey<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

// implicit type conversions for convenient API
impl<'a> From<&'a Path> for AssetKey<'a> {
    fn from(p: &'a Path) -> Self {
        Self {
            path: Cow::from(p.to_path_buf()),
            scheme: None,
        }
    }
}

impl<'a> From<&'a str> for AssetKey<'static> {
    fn from(s: &'a str) -> Self {
        s.parse().unwrap()
    }
}

impl<'a> From<PathBuf> for AssetKey<'a> {
    fn from(p: PathBuf) -> Self {
        Self::from_path(p)
    }
}

impl<'a> Into<PathBuf> for AssetKey<'a> {
    fn into(self) -> PathBuf {
        self.path.into_owned()
    }
}

impl Into<AssetKey<'static>> for &'static AssetKey<'static> {
    fn into(self) -> AssetKey<'static> {
        self.clone()
    }
}

impl<'a> AssetKey<'a> {
    pub fn new<P1, P2>(p: P1, s: Option<P2>) -> Self
    where
        P1: Into<Cow<'a, Path>> + Clone,
        P2: Into<Cow<'a, Path>> + Clone,
    {
        AssetKey {
            scheme: s.map(|x| x.into()),
            path: p.into(),
        }
    }

    // FIXME:
    fn to_static(&self) -> AssetKey<'static> {
        AssetKey {
            scheme: self.scheme.clone().map(|s| Cow::Owned(s.into_owned())),
            path: Cow::Owned(self.path.clone().into_owned()),
        }
    }
}

/// `AssetKey::to_string` never fails
impl<'a> fmt::Display for AssetKey<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scheme) = &self.scheme {
            write!(f, "{}:{}", scheme.display(), self.path.display())?;
        } else {
            write!(f, "{}", self.path.display())?;
        }

        Ok(())
    }
}

impl<'a> str::FromStr for AssetKey<'a> {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = if let Some(colon) = s.bytes().position(|b| b == b':') {
            Self {
                path: if s.len() == colon + 1 {
                    Path::new("./").into()
                } else {
                    Cow::Owned(PathBuf::from(s[colon + 1..].to_string()))
                },
                scheme: Some(Cow::Owned(PathBuf::from(s[0..colon].to_string()))),
            }
        } else {
            Self {
                path: Cow::Owned(PathBuf::from(s)),
                scheme: None,
            }
        };

        Ok(key)
    }
}

/// Explicit type conversions
impl<'a> AssetKey<'a> {
    pub fn from_path(p: impl Into<Cow<'a, Path>>) -> Self {
        AssetKey {
            path: p.into(),
            scheme: None,
        }
    }
}

impl AssetKey<'static> {
    /**
    Create static asset key from static path

    `&'static Path` can be created as this:

    ```no_run
    #![feature(const_raw_ptr_deref)]
    use std::{ffi::OsStr, path::Path};

    const fn as_path(s:&'static str) -> &'static Path {
        unsafe { &*(s as *const str as *const OsStr as *const Path) }
    }
    ```
    */
    pub const fn new_const(path: &'static Path, scheme: Option<&'static Path>) -> Self {
        AssetKey {
            path: Cow::Borrowed(path),
            scheme: if let Some(s) = scheme {
                Some(Cow::Borrowed(s))
            } else {
                None
            },
        }
    }
}

/// Prefer [`AssetKey::new_const`]
#[derive(Clone, Copy)]
pub struct StaticAssetKey {
    pub path: &'static str,
    pub scheme: Option<&'static str>,
}

impl<'a> Into<AssetKey<'a>> for StaticAssetKey {
    fn into(self) -> AssetKey<'a> {
        AssetKey {
            path: Cow::Borrowed(self.path.as_ref()),
            scheme: self.scheme.map(|s| Cow::Borrowed(s.as_ref())),
        }
    }
}

/// [`AssetKey`] resolved as an absolute path
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ResolvedPath {
    path: PathBuf,
}

/// Resolves asset path to absolute path
#[derive(Debug)]
struct Resolver {
    root: PathBuf,
    // schemes: Vec<(String, PathBuf)>,
}

impl Resolver {
    // TODO: return AbsolutePath newtype
    pub fn resolve<'a>(&self, key: impl Into<AssetKey<'a>>) -> PathBuf {
        let key: AssetKey<'a> = key.into();
        // TODO: handle scheme
        self.root.join(key.path)
    }
}

/// All assets on memory
#[derive(Debug)]
pub struct AssetCache {
    resolver: Resolver,
    caches: HashMap<TypeId, Box<dyn FreeUnused>>,
}

impl AssetCache {
    pub fn with_root(root: PathBuf) -> Self {
        assert!(root.is_absolute());
        log::trace!("Given asset root {}", root.display());

        Self {
            resolver: Resolver { root },
            caches: HashMap::new(),
        }
    }

    pub fn free_unused(&mut self) {
        for cache in &mut self.caches.values_mut() {
            cache.free_unused();
        }
    }

    pub fn add_cache<T: AssetItem>(&mut self, loader: T::Loader) {
        let cache = AssetCacheT::<T>::new(loader);
        let existing = self.caches.insert(TypeId::of::<T>(), Box::new(cache));
        assert!(existing.is_none(), "Duplicate cache creation");
    }

    fn cache_mut<T: AssetItem>(&mut self) -> Option<&mut AssetCacheT<T>> {
        let boxed = self.caches.get_mut(&TypeId::of::<T>()).unwrap();
        boxed.downcast_mut::<AssetCacheT<T>>()
    }

    pub fn load_sync<'key, T: AssetItem, K: Into<AssetKey<'key>>>(
        &mut self,
        key: K,
    ) -> Result<Asset<T>> {
        self.load_sync_impl(key, false)
    }

    pub fn load_sync_preserve<'key, T: AssetItem, K: Into<AssetKey<'key>>>(
        &mut self,
        key: K,
    ) -> Result<Asset<T>> {
        self.load_sync_impl(key, true)
    }

    fn load_sync_impl<'key, T: AssetItem, K: Into<AssetKey<'key>>>(
        &mut self,
        key: K,
        preserve: bool,
    ) -> Result<Asset<T>> {
        let key: AssetKey<'key> = key.into();
        let serde_repr = key.to_static();
        let entry_id = self.resolve(key.clone());

        let cache_t = self.cache_mut::<T>().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Non-existing asset cache for type {}",
                    std::any::type_name::<T>()
                ),
            )
        })?;

        if let Some(asset) = cache_t.lookup_cache(&entry_id) {
            log::trace!(
                "(cache found for `{}` of type `{}`)",
                entry_id.display(),
                any::type_name::<T>()
            );

            Ok(asset)
        } else {
            log::debug!(
                "loading asset `{}` of type `{}`",
                entry_id.display(),
                any::type_name::<T>()
            );

            let loader = Rc::clone(&cache_t.loader);
            drop(cache_t);

            let load_path = entry_id.clone();
            let item = loader.load(&load_path, self)?;
            let cache_t = self.cache_mut::<T>().unwrap();

            let asset = cache_t.insert(entry_id, item, preserve, load_path, serde_repr);

            Ok(asset)
        }
    }
}

/// Aseet path handling
impl AssetCache {
    /// Resolves asset path into an absolute path
    // pub fn resolve<'key>(&self, key: impl Into<AssetKey<'key>>) -> PathBuf {
    pub fn resolve<'key>(&self, key: impl Into<AssetKey<'key>>) -> PathBuf {
        self.resolver.resolve(key)
    }

    pub fn read_to_string<'key>(&self, key: impl Into<AssetKey<'key>>) -> io::Result<String> {
        let path = self.resolve(key);
        fs::read_to_string(&path)
    }

    /// Deserialize asset file as RON
    pub fn load_ron<'key, T: serde::de::DeserializeOwned>(
        &self,
        key: impl Into<AssetKey<'key>>,
    ) -> anyhow::Result<T> {
        let path = self.resolve(key);
        let s = fs::read_to_string(&path)
            .map_err(anyhow::Error::msg)
            .with_context(|| {
                anyhow::anyhow!("Unable to read asset file at `{}`", path.display())
            })?;

        ron::de::from_str::<T>(&s)
            .map_err(anyhow::Error::msg)
            .with_context(|| {
                format!(
                    "Unable deserialize `{}` for type {}",
                    path.display(),
                    std::any::type_name::<T>()
                )
            })
    }
}

/// Access to an [`AssetItem`] with metadata
#[derive(Debug)]
struct AssetCacheEntry<T: AssetItem> {
    /// FIXME: Don't use absolute as identity
    id: PathBuf,
    /// Used for debug display. `Rc` since it's at least shared with the asset
    load_path: Rc<PathBuf>,
    asset: Asset<T>,
}

/// Cache of specific [`AssetItem`] type items. This is hidden from user, so it can be low-level item
#[derive(Debug)]
struct AssetCacheT<T: AssetItem> {
    entries: Vec<AssetCacheEntry<T>>,
    loader: Rc<T::Loader>,
    gen: Gen,
}

impl<T: AssetItem> AssetCacheT<T> {
    pub(crate) fn new(loader: T::Loader) -> Self {
        Self {
            entries: Vec::with_capacity(16),
            loader: Rc::new(loader),
            gen: 0,
        }
    }

    pub(crate) fn lookup_cache(&mut self, entry_id: &PathBuf) -> Option<Asset<T>> {
        self.entries
            .iter()
            .find(|entry| &entry.id == entry_id)
            .map(|entry| entry.asset.clone())
    }

    pub(crate) fn insert(
        &mut self,
        id: PathBuf,
        item: T,
        preserved: bool,
        load_path: PathBuf,
        serde_repr: AssetKey<'static>,
    ) -> Asset<T> {
        let load_path = Rc::new(load_path);

        let asset = Asset {
            item: Some(Arc::new(Mutex::new(item))),
            preserved: Arc::new(Mutex::new(preserved)),
            load_path: Rc::clone(&load_path),
            serde_repr: Rc::new(serde_repr),
            identity: self.gen,
        };
        self.gen += 1;

        let entry = AssetCacheEntry {
            id,
            load_path: Rc::clone(&load_path),
            asset: asset.clone(),
        };
        self.entries.push(entry);

        asset
    }
}

/// Upcast of [`AssetCacheT`]
trait FreeUnused: fmt::Debug + Downcast {
    fn free_unused(&mut self);
}

impl_downcast!(FreeUnused);

impl<T: AssetItem> FreeUnused for AssetCacheT<T> {
    fn free_unused(&mut self) {
        let mut i = 0;
        let mut len = self.entries.len();
        while i < len {
            let entry = &mut self.entries[i];
            if let Some(item) = &entry.asset.item {
                // if the asset entry is the only owner
                // and it's not stated to be preserved
                if Arc::strong_count(item) == 1 && !*entry.asset.preserved.lock().unwrap() {
                    log::debug!(
                        "free asset at `{}` in slot `{}` of cache for type `{}`",
                        self.entries[i].load_path.display(),
                        i,
                        std::any::type_name::<T>(),
                    );
                    self.entries.remove(i);
                    len -= 1;
                }
            }
            i += 1;
        }
    }
}

/// Deserialize assets without making duplicates using thread-local variable
#[derive(Debug)]
struct AssetDeState {
    cache: UnsafeCell<AssetCache>,
}

impl AssetDeState {
    // fn cache(&self) -> &AssetCache {
    //     unsafe { &*(self.cache.get()) }
    // }

    fn cache_mut(&self) -> &mut AssetCache {
        unsafe { &mut *(self.cache.get()) }
    }
}

/// TODO: Just use thread_local!
static mut DE_STATE: OnceCell<AssetDeState> = OnceCell::new();

/// Run a procedure in a guarded scope where [`Asset`] can deserialize
pub fn guarded<T>(original_cache: &mut AssetCache, proc: impl FnOnce(&mut AssetCache) -> T) -> T {
    // take the owner ship of original cache
    let mut cache = AssetCache::with_root(PathBuf::new());
    mem::swap(original_cache, &mut cache);
    // and wrap it in `UnsafeCell`:
    let cache = UnsafeCell::new(cache);

    unsafe {
        DE_STATE
            .set(AssetDeState { cache })
            .unwrap_or_else(|_old_value| {
                unreachable!("DE_STATE is guarded by snow2d::asset::with_cache")
            });
    }

    let res = proc(original_cache);

    // give the AssetState back to the original place
    let mut cache = unsafe {
        match DE_STATE.take() {
            Some(state) => state.cache.into_inner(),
            None => unreachable!(),
        }
    };

    mem::swap(&mut cache, original_cache);

    res
}

impl<T: AssetItem> Serialize for Asset<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize as PathBuf
        self.serde_repr.serialize(serializer)
    }
}

/// # Safety
/// `Asset::deserialize` the only place `DE_STATE` is used and performs one-shot
/// access to [`AssetDeState`]. So there's no overlapping borrow and it sounds!
impl<'de, T: AssetItem> Deserialize<'de> for Asset<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserialize as AssetKey
        let key = <AssetKey as Deserialize>::deserialize(deserializer)
            .map_err(|e| de::Error::custom(format!("Unable to load asset as `PathBuf`: {}", e)))?;

        // then load asset
        let state = unsafe {
            DE_STATE
                .get_mut()
                .ok_or_else(|| "Unable to find asset cache")
                .unwrap()
        };

        // The only access to `DE_STATE`, which is one-shot and never overlap with other borrrow
        let cache = state.cache_mut();
        let item = cache.load_sync(key.clone()).map_err(|e| {
            de::Error::custom(format!("Error while loading asset at `{}`: {}", key, e))
        })?;
        drop(cache);

        Ok(item)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn key() {
        let key: AssetKey = "schemed:path/to/asset".parse().unwrap();

        let correct = AssetKey {
            path: Cow::Owned(PathBuf::from("path/to/asset")),
            scheme: Some(Cow::Owned(PathBuf::from("schemed"))),
        };

        assert_eq!(key, correct);

        let s = ron::ser::to_string(&key).unwrap();
        assert_eq!(s, "\"schemed:path/to/asset\"");

        let d: AssetKey = ron::de::from_str(&s).unwrap();
        assert_eq!(d, correct);
    }
}
