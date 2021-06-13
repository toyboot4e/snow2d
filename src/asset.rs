/*!
Asset management

# Asset types

[`Asset<T>`] is a shared reference of an asset item. Any type of `Asset` is stored in
[`AssetCache`].

# Asset directory

Asset directory is assumed to be at `manifest_dir/assets`. [`AssetKey`] is a relative path from
the asset directory.

# Serde support

Bind your [`AssetCache`] to [`AssetDeState`] while deserializing.

Every [`Asset`] is serialized as [`PathBuf`] and deserialiezd as [`Asset`]. Since [`Asset`] is a
shared pointer, we need to take care to not create duplicates. But `serde` doesn't let us to use
states while serializing/deserializing. So we need a thread-local pointer for deserialization.

# TODOs

* async loading
* hot reloading (tiled map, actor image, etc.)
* `Asset` implements `Deref`, but they don't implement traits that the underlying data implements
*/

#![allow(dead_code)]

/// `std::io::Result` re-exported
///
/// ---
pub use std::io::Result;

use std::{
    any::TypeId,
    borrow::Cow,
    collections::HashMap,
    fmt, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use downcast_rs::{impl_downcast, Downcast};
use once_cell::sync::OnceCell;
use serde::{de::Deserializer, ser::Serializer, Deserialize, Serialize};

use crate::utils::Cheat;

/// Generational index or identity of assets
type Gen = u32;

// TODO: scheme support
// /// `"scheme:path"` or `"relative_path"`
// #[derive(Debug, Clone, PartialEq, Eq, Hash, Inspect)]
// pub struct StringWithScheme {
//     raw: String,
//     /// Byte offset of `:` character
//     scheme_offset: Option<usize>,
// }
//
// /// Maps scheme to relative path from asset root directory
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct SchemeHolder {
//     schemes: Vec<(String, String)>,
// }
//
// impl StringWithScheme {
//     pub fn as_scheme(&self) -> Option<&str> {
//         self.scheme_offset.map(|offset| &self.raw[offset..])
//     }
//
//     pub fn as_body(&self) -> &str {
//         let offset = self.scheme_offset.map(|offset| offset + 1).unwrap_or(0);
//         &self.raw[offset..]
//     }
// }

/// Data that can be used as an asset
pub trait AssetItem: fmt::Debug + Sized + 'static {
    type Loader: AssetLoader<Item = Self>;
}

/// How to load an [`AssetItem`]
pub trait AssetLoader: fmt::Debug + Sized + 'static {
    type Item: AssetItem;
    fn load(&mut self, path: &Path, cache: &mut AssetCache) -> Result<Self::Item>;
}

/// Shared ownership of an asset item
#[derive(Debug)]
pub struct Asset<T: AssetItem> {
    item: Option<Arc<Mutex<T>>>,
    preserved: Arc<Mutex<bool>>,
    // constant data is not put in shared memory:
    path: Rc<PathBuf>,
    identity: Gen,
}

impl<T: AssetItem> Clone for Asset<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.as_ref().map(|x| Arc::clone(x)),
            preserved: Arc::clone(&self.preserved),
            path: Rc::clone(&self.path),
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
        self.path.as_ref()
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

/// Special URI for an asset (c.f. [`AssetCache::resolve`])
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetKey<'a> {
    path: Cow<'a, Path>,
    scheme: Option<Cow<'a, Path>>,
}

impl<'a, 'de> Deserialize<'de> for AssetKey<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let x = String::deserialize(deserializer)?;
        let key = Self::parse(x);
        // TODO: validate the key?
        Ok(key)
    }
}

impl<'a> Serialize for AssetKey<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string_repr();
        s.serialize(serializer)
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
        Self::parse(s)
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
            path: p.into(),
            scheme: s.map(|x| x.into()),
        }
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

    /// Parses a schemed string in syntax `scheme:relative/path`
    pub fn parse<'b>(s: impl Into<Cow<'b, str>>) -> Self {
        let s = s.into();

        let s_ref = s.as_ref();
        if let Some(colon) = s_ref.bytes().position(|b| b == b':') {
            Self {
                path: if s.len() == colon + 1 {
                    Path::new("./").into()
                } else {
                    Cow::Owned(PathBuf::from(s_ref[colon + 1..].to_string()))
                },
                scheme: Some(Cow::Owned(PathBuf::from(s_ref[0..colon].to_string()))),
            }
        } else {
            Self {
                path: Cow::Owned(PathBuf::from(s.into_owned())),
                scheme: None,
            }
        }
    }

    pub fn to_string_repr(&self) -> String {
        if let Some(scheme) = &self.scheme {
            format!("{}:{}", scheme.display(), self.path.display())
        } else {
            format!("{}", self.path.display())
        }
    }
}

impl AssetKey<'static> {
    /**
    Create static asset key with static path

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

/// Key to load asset (allocated statically)
///
/// See also: [`AssetKey::new_const`]
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

/// [`AssetKey`] resolved to be a [`PathBuf`]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ResolvedPath {
    path: PathBuf,
}

/// Access to an [`AssetItem`] with metadata
#[derive(Debug)]
struct AssetCacheEntry<T: AssetItem> {
    id: ResolvedPath,
    path: Rc<PathBuf>,
    asset: Asset<T>,
}

/// Cache of specific [`AssetItem`] type items
#[derive(Debug)]
struct AssetCacheT<T: AssetItem> {
    any_cache: Cheat<AssetCache>,
    entries: Vec<AssetCacheEntry<T>>,
    loader: T::Loader,
    gen: Gen,
}

/// All of the assets
#[derive(Debug)]
pub struct AssetCache {
    caches: HashMap<TypeId, Box<dyn FreeUnused>>,
}

impl<T: AssetItem> AssetCacheT<T> {
    pub fn new(loader: T::Loader) -> Self {
        Self {
            any_cache: unsafe { Cheat::null() },
            entries: Vec::with_capacity(16),
            loader,
            gen: 0,
        }
    }

    pub fn load_sync<'a>(&mut self, key: impl Into<AssetKey<'a>>) -> Result<Asset<T>> {
        let key = key.into();

        // TODO: remove this code on release build
        let repr = key.to_string_repr();

        let id = ResolvedPath {
            path: self.any_cache.resolve(key),
        };

        if let Some(entry) = self.entries.iter().find(|a| a.id == id) {
            log::trace!(
                "(cache found for `{}` of type `{}`)",
                repr,
                std::any::type_name::<T>()
            );

            Ok(entry.asset.clone())
        } else {
            log::debug!(
                "loading asset `{}` of type `{}`",
                repr,
                std::any::type_name::<T>()
            );

            self.load_new_sync(id)
        }
    }

    pub fn load_sync_preserve<'a>(&mut self, key: impl Into<AssetKey<'a>>) -> Result<Asset<T>> {
        let mut res = self.load_sync(key);
        if let Ok(asset) = res.as_mut() {
            asset.set_preserved(true);
        }
        res
    }

    fn load_new_sync(&mut self, id: ResolvedPath) -> Result<Asset<T>> {
        let key = AssetKey::from_path(id.path.clone());
        let path = Rc::new(self.any_cache.resolve(key));

        let asset = Asset {
            item: {
                let item = self.loader.load(&path, &mut self.any_cache)?;
                Some(Arc::new(Mutex::new(item)))
            },
            preserved: Arc::new(Mutex::new(false)),
            path: Rc::clone(&path),
            identity: self.gen,
        };
        self.gen += 1;

        let entry = AssetCacheEntry {
            id,
            path: Rc::clone(&path),
            asset: asset.clone(),
        };
        self.entries.push(entry);

        Ok(asset)
    }
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            caches: HashMap::with_capacity(16),
        }
    }

    pub fn free_unused(&mut self) {
        for cache in &mut self.caches.values_mut() {
            cache.free_unused();
        }
    }

    pub fn add_cache<T: AssetItem>(&mut self, loader: T::Loader) {
        let mut cache = AssetCacheT::<T>::new(loader);
        cache.any_cache = unsafe { Cheat::new(self) };
        self.caches.insert(TypeId::of::<T>(), Box::new(cache));
    }

    fn cache_mut<T: AssetItem>(&mut self) -> Option<&mut AssetCacheT<T>> {
        let boxed = self.caches.get_mut(&TypeId::of::<T>()).unwrap();
        boxed.downcast_mut::<AssetCacheT<T>>()
    }

    pub fn load_sync<'a, T: AssetItem, K: Into<AssetKey<'a>>>(
        &mut self,
        key: K,
    ) -> Result<Asset<T>> {
        let key = key.into();
        self.cache_mut::<T>()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "Non-existing asset cache for type {}",
                        std::any::type_name::<T>()
                    ),
                )
            })?
            .load_sync(key)
    }

    pub fn load_sync_preserve<'a, T: AssetItem, K: Into<AssetKey<'a>>>(
        &mut self,
        key: K,
    ) -> Result<Asset<T>> {
        let mut res = self.load_sync(key);
        if let Ok(asset) = res.as_mut() {
            asset.set_preserved(true);
        }
        res
    }
}

/// Aseet path handling
impl AssetCache {
    /// Resolves asset path into an absolute path
    pub fn resolve<'a>(&self, key: impl Into<AssetKey<'a>>) -> PathBuf {
        // TODO: runtime asset root detection
        let proj_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let asset_root = PathBuf::from(proj_root).join("assets");
        asset_root.join(key.into().path)
    }

    /// Deserialize asset path as a RON file
    pub fn deserialize_ron<'a, T: serde::de::DeserializeOwned>(
        &self,
        key: impl Into<AssetKey<'a>>,
    ) -> anyhow::Result<T> {
        use std::fs;

        let path = self.resolve(key);
        log::trace!("deserializing `{}`", path.display());

        let s = fs::read_to_string(&path)
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("Unable to read asset file at `{}`", path.display()))?;
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
                        self.entries[i].path.display(),
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
pub struct AssetDeState {
    cache: Cheat<AssetCache>,
}

static mut DE_STATE: OnceCell<AssetDeState> = OnceCell::new();

impl AssetDeState {
    /// Run a procedure with global access to asset cache
    pub fn run<T>(cache: &mut AssetCache, proc: impl FnOnce(&mut AssetCache) -> T) -> T {
        unsafe {
            DE_STATE
                .set(Self {
                    cache: Cheat::new(cache),
                })
                .unwrap_or_else(|_old_value| {
                    unreachable!("DE_STATE is guarded by AssetDeState::run")
                });
        }

        let res = proc(cache);

        unsafe {
            assert!(DE_STATE.take().is_some());
        }

        res
    }
}

impl<T: AssetItem> Serialize for Asset<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize as PathBuf
        self.path.serialize(serializer)
    }
}

// TODO: Ensure to not panic while deserializing
impl<'de, T: AssetItem> Deserialize<'de> for Asset<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserialize as PathBuf
        let path = <PathBuf as Deserialize>::deserialize(deserializer)
            .map_err(|e| format!("Unable to load asset as `PathBuf`: {}", e))
            .unwrap();

        // then load asset
        let state = unsafe {
            DE_STATE
                .get_mut()
                .ok_or_else(|| "Unable to find asset cache")
                .unwrap()
        };

        let item = state
            .cache
            .load_sync(AssetKey::from_path(&path))
            .map_err(|e| format!("Error while loading asset at `{}`: {}", path.display(), e))
            .unwrap();

        Ok(item)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn key() {
        let key = AssetKey::parse("schemed:path/to/asset");

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
