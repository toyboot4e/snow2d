/*!
[Type object][to] pattern with `serde` support

[to]: https://gameprogrammingpatterns.com/type-object.html

The idea is to have a [static storage][TypeObjectStorage] of [`TypeObjectMap`] for each [`TypeObject`].
*/

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    marker::PhantomData,
    rc::Rc,
};

use derivative::Derivative;
use once_cell::sync::OnceCell;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use snow2d_derive::TypeObject;

use crate::{
    asset::{AssetCache, AssetKey},
    utils::Inspect,
};

/// Marker for data that define "type" of instances (type objects). Type objects are stored in
/// static storage.
pub trait TypeObject: std::fmt::Debug + Sized {
    fn from_type_key(key: &TypeObjectId<Self>) -> anyhow::Result<Rc<Self>>
    where
        Self: 'static,
    {
        TypeObjectStorage::get_type_object(key)
            .ok_or_else(|| anyhow::anyhow!(format!("Unable to find type object for {:?}", key)))
    }
}

/// Id of [`TypeObject`], which can be used to retrieve the [`TypeObject`] through global storage
#[derive(Derivative, Inspect)]
#[derivative(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeObjectId<T: TypeObject> {
    /// TODO: use `Cow` and add lifetime?
    key: String,
    _ty: PhantomData<fn() -> T>,
}

impl<T: TypeObject> fmt::Display for TypeObjectId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.key.fmt(f)
    }
}

impl<'a, T: TypeObject> From<&'a str> for TypeObjectId<T> {
    fn from(s: &'a str) -> Self {
        TypeObjectId {
            key: s.to_string(),
            _ty: PhantomData,
        }
    }
}

impl<'de, T: TypeObject> serde::de::Deserialize<'de> for TypeObjectId<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let key = <String as serde::de::Deserialize>::deserialize(deserializer)?;
        Ok(Self {
            key,
            _ty: PhantomData,
        })
    }
}

impl<T: TypeObject> serde::ser::Serialize for TypeObjectId<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.key.serialize(serializer)
    }
}

impl<T: TypeObject> TypeObjectId<T> {
    /// Creates type object from raw ID
    pub fn from_raw(s: String) -> Self {
        Self {
            key: s,
            _ty: PhantomData,
        }
    }

    pub fn raw(&self) -> &str {
        &self.key
    }

    /// Tries to retrieve the target type object from global storage
    pub fn try_retrieve(&self) -> Option<Rc<T>>
    where
        T: 'static,
    {
        let storage = TypeObjectStorage::get_map::<T>()?;
        storage.get(self)
    }
}

pub fn storage_builder<'c>(cache: &'c mut AssetCache) -> Result<TypeObjectStorageBuilder<'c>, ()> {
    TypeObjectStorage::init().map_err(|_e| ())?;
    Ok(TypeObjectStorageBuilder { cache })
}

/// Utility for initializing static [`TypeObjectStorage`]
#[derive(Debug)]
pub struct TypeObjectStorageBuilder<'c> {
    cache: &'c mut AssetCache,
}

impl<'c> TypeObjectStorageBuilder<'c> {
    pub fn add<'a, T: TypeObject + 'static + DeserializeOwned, U: Into<AssetKey<'a>>>(
        &mut self,
        key: U,
    ) -> anyhow::Result<&mut Self> {
        log::trace!(
            "registering type object storage for type `{}`",
            std::any::type_name::<T>()
        );

        TypeObjectStorage::register_type_objects::<T, U>(key, &mut self.cache).unwrap();

        Ok(self)
    }
}

impl<'c> Drop for TypeObjectStorageBuilder<'c> {
    fn drop(&mut self) {
        log::trace!("loaded type objects");
    }
}

/// Static storage of type objects
#[derive(Debug)]
pub struct TypeObjectStorage {
    maps: HashMap<TypeId, Box<dyn Any>>,
}

static mut STORAGE: OnceCell<TypeObjectStorage> = OnceCell::new();

impl TypeObjectStorage {
    fn init() -> Result<(), Self> {
        unsafe {
            STORAGE.set(TypeObjectStorage {
                maps: HashMap::with_capacity(16),
            })
        }
    }

    fn register_type_objects<
        'a,
        T: TypeObject + 'static + DeserializeOwned,
        U: Into<AssetKey<'a>>,
    >(
        key: U,
        cache: &mut AssetCache,
    ) -> anyhow::Result<()> {
        unsafe {
            let s = STORAGE
                .get_mut()
                .expect("TypeObjectStorage is not initialized");

            // we might accesst oa
            let map: HashMap<TypeObjectId<T>, T> = cache.load_ron(key)?;

            let map: HashMap<TypeObjectId<T>, Rc<T>> = map
                .into_iter()
                .map(|(key, value)| (key, Rc::new(value)))
                .collect::<HashMap<_, _>>();

            let dup = s
                .maps
                .insert(TypeId::of::<T>(), Box::new(TypeObjectMap { data: map }));
            anyhow::ensure!(
                dup.is_none(),
                "Registring type objects twice for type `{}`",
                std::any::type_name::<T>(),
            );

            Ok(())
        }
    }

    fn get_any<T: TypeObject + 'static>() -> &'static Box<dyn Any> {
        unsafe {
            let s = STORAGE.get().expect("TypeObjectStorage is not initialized");

            s.maps.get(&TypeId::of::<T>()).unwrap_or_else(|| {
                panic!(
                    "No TypeObjectMap found for type `{}`",
                    std::any::type_name::<T>()
                )
            })
        }
    }

    /// TODO: Use guard instead
    pub fn get_map<T: TypeObject>() -> Option<&'static TypeObjectMap<T>> {
        Self::get_any::<T>().downcast_ref::<TypeObjectMap<T>>()
    }

    pub fn get_type_object<T: TypeObject + 'static>(id: &TypeObjectId<T>) -> Option<Rc<T>> {
        let map = Self::get_map::<T>()?;
        map.get(id)
    }
}

/// Maps [`TypeObjectId`] to [`TypeObject`]
pub struct TypeObjectMap<T: TypeObject> {
    // TODO: use Pool, not `Rc`
    data: HashMap<TypeObjectId<T>, Rc<T>>,
}

impl<T: TypeObject> TypeObjectMap<T> {
    pub fn get(&self, id: &TypeObjectId<T>) -> Option<Rc<T>> {
        self.data.get(id).map(|rc| Rc::clone(rc))
    }
}

/// `Reference` | `Embedded` representation of a type object in field
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SerdeRepr<T: TypeObject> {
    /// Id of a type object, which can be converted to [`Rc<T>`]
    Reference(TypeObjectId<T>),
    /// Owned type object
    Embedded(T),
}

impl<T: TypeObject> SerdeRepr<T> {
    /// Returns `None` if we can't retrieve `T`
    pub fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> Option<U>
    where
        T: 'static,
    {
        Some(match self {
            SerdeRepr::Reference(id) => f(id.try_retrieve()?.as_ref()),
            SerdeRepr::Embedded(t) => f(t),
        })
    }
}

pub use snow2d_derive::SerdeViaTyObj;

/// Internal utility to implement `From` and `Into` between [`SerdeRepr`] and target data type
pub trait SerdeViaTyObj {
    type TypeObject: TypeObject + 'static;

    fn _repr_mut(&mut self) -> Option<&mut SerdeRepr<Self::TypeObject>> {
        None
    }

    fn from_tyobj(obj: &Self::TypeObject) -> Self;

    fn _from_tyobj_with_id(obj: &Self::TypeObject, id: &TypeObjectId<Self::TypeObject>) -> Self
    where
        Self: Sized,
    {
        let mut target = Self::from_tyobj(&obj);
        if let Some(repr) = target._repr_mut() {
            *repr = SerdeRepr::Reference(id.clone());
        }
        target
    }

    /// `Into<SerdeRepr<TargetType>` implementation
    fn into_tyobj_repr(self) -> Option<SerdeRepr<Self::TypeObject>>
    where
        Self: Sized,
    {
        None
    }

    /// `From<SerdeRepr<TargetType>>` implementation
    fn from_tyobj_repr(repr: SerdeRepr<Self::TypeObject>) -> Self
    where
        Self: Sized,
    {
        match repr {
            // no ID
            SerdeRepr::Embedded(tyobj) => Self::from_tyobj(&tyobj),
            // some ID
            SerdeRepr::Reference(id) => {
                Self::_from_tyobj_with_id(id.try_retrieve().unwrap().as_ref(), &id)
            }
        }
    }
}
