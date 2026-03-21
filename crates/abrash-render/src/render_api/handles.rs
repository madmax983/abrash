//! Opaque resource handles and typed resource pool.
//!
//! Handles are lightweight IDs that reference renderer-owned resources.
//! They are type-safe (`MeshHandle` can't be used where `TextureHandle` is expected)
//! and generation-checked to detect use-after-free.

use std::marker::PhantomData;

/// Generation counter to detect stale handles.
type Generation = u32;

/// A typed, generation-checked resource handle.
///
/// The `T` phantom type prevents mixing handle types at compile time.
/// The generation field detects use-after-free at runtime.
///
/// # Examples
///
/// ```
/// use abrash_render::render_api::{MeshHandle, TextureHandle};
/// use abrash_render::render_api::Handle;
///
/// let mesh_h: MeshHandle = Handle::from_raw_parts(0, 1);
/// let tex_h: TextureHandle = Handle::from_raw_parts(0, 1);
///
/// // Even though both handles have index 0 and generation 1, they are distinct types.
/// // The compiler will reject passing `mesh_h` to a function expecting `TextureHandle`.
/// ```
pub struct Handle<T> {
    pub(crate) index: u32,
    pub(crate) generation: Generation,
    pub(crate) _marker: PhantomData<fn() -> T>,
}

// Manual impls so bounds track the handle's properties, not T's.
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

impl<T> Handle<T> {
    /// Create a handle from raw parts.
    ///
    /// This is primarily used by backend crates that own their own resource pools but
    /// share the typed handle API.
    #[must_use]
    pub const fn from_raw_parts(index: u32, generation: Generation) -> Self {
        Self {
            index,
            generation,
            _marker: PhantomData,
        }
    }

    /// Return the slot index referenced by this handle.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Return the generation stored in this handle.
    #[must_use]
    pub const fn generation(self) -> Generation {
        self.generation
    }

    /// Return the raw `(index, generation)` tuple.
    #[must_use]
    pub const fn raw_parts(self) -> (u32, Generation) {
        (self.index, self.generation)
    }

    /// Create a new handle (internal use only).
    pub(crate) const fn new(index: u32, generation: Generation) -> Self {
        Self::from_raw_parts(index, generation)
    }
}

/// Marker type for mesh resources.
#[derive(Debug)]
pub struct MeshResource;
/// Marker type for texture resources.
#[derive(Debug)]
pub struct TextureResource;
/// Marker type for material resources.
#[derive(Debug)]
pub struct MaterialResource;

/// Handle to a mesh uploaded to the renderer.
pub type MeshHandle = Handle<MeshResource>;
/// Handle to a texture uploaded to the renderer.
pub type TextureHandle = Handle<TextureResource>;
/// Handle to a material definition in the renderer.
pub type MaterialHandle = Handle<MaterialResource>;

/// A generational arena for storing resources behind handles.
///
/// Supports O(1) insert, lookup, and remove with generation-based
/// stale handle detection.
///
/// # Examples
///
/// ```
/// use abrash_render::render_api::{ResourcePool, Handle};
///
/// let mut pool: ResourcePool<String> = ResourcePool::new();
///
/// // Insert a resource, get a handle back.
/// let handle: Handle<String> = pool.insert("Hello".to_string());
/// assert_eq!(pool.get(handle), Some(&"Hello".to_string()));
///
/// // Remove the resource. The pool reclaims the memory slot and increments the generation.
/// let removed = pool.remove(handle);
/// assert_eq!(removed, Some("Hello".to_string()));
///
/// // Trying to use the old handle now returns `None` safely.
/// // We've prevented a use-after-free!
/// assert_eq!(pool.get(handle), None);
/// ```
pub struct ResourcePool<T> {
    entries: Vec<PoolEntry<T>>,
    free_list: Vec<u32>,
}

enum PoolEntry<T> {
    Occupied { value: T, generation: Generation },
    Vacant { generation: Generation },
}

impl<T> ResourcePool<T> {
    /// Create a new empty resource pool.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_list: Vec::new(),
        }
    }

    /// Insert a resource and return its handle.
    pub fn insert(&mut self, value: T) -> Handle<T> {
        if let Some(index) = self.free_list.pop() {
            let entry = &mut self.entries[index as usize];
            let generation = match entry {
                PoolEntry::Vacant { generation } => *generation,
                PoolEntry::Occupied { .. } => unreachable!("free list pointed to occupied slot"),
            };
            *entry = PoolEntry::Occupied { value, generation };
            Handle::new(index, generation)
        } else {
            let index = self.entries.len() as u32;
            let generation = 0;
            self.entries.push(PoolEntry::Occupied { value, generation });
            Handle::new(index, generation)
        }
    }

    /// Look up a resource by handle. Returns `None` if handle is stale or invalid.
    #[must_use]
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.entries
            .get(handle.index as usize)
            .and_then(|entry| match entry {
                PoolEntry::Occupied { value, generation } if *generation == handle.generation => {
                    Some(value)
                }
                _ => None,
            })
    }

    /// Mutable lookup by handle.
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.entries
            .get_mut(handle.index as usize)
            .and_then(|entry| match entry {
                PoolEntry::Occupied { value, generation } if *generation == handle.generation => {
                    Some(value)
                }
                _ => None,
            })
    }

    /// Remove a resource and return it. Increments generation to invalidate old handles.
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let entry = self.entries.get_mut(handle.index as usize)?;
        match entry {
            PoolEntry::Occupied { generation, .. } if *generation == handle.generation => {
                let new_gen = *generation + 1;
                let old = std::mem::replace(
                    entry,
                    PoolEntry::Vacant {
                        generation: new_gen,
                    },
                );
                self.free_list.push(handle.index);
                match old {
                    PoolEntry::Occupied { value, .. } => Some(value),
                    PoolEntry::Vacant { .. } => unreachable!(),
                }
            }
            _ => None,
        }
    }
}

impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_type_safety() {
        let mesh: MeshHandle = Handle::new(0, 0);
        let tex: TextureHandle = Handle::new(0, 0);
        assert_eq!(mesh.index, tex.index);
    }

    #[test]
    fn test_handle_raw_parts_round_trip() {
        let handle: MeshHandle = MeshHandle::from_raw_parts(7, 3);

        assert_eq!(handle.index(), 7);
        assert_eq!(handle.generation(), 3);
        assert_eq!(handle.raw_parts(), (7, 3));
    }

    #[test]
    fn test_pool_insert_and_get() {
        let mut pool: ResourcePool<String> = ResourcePool::new();
        let h1 = pool.insert("hello".to_string());
        let h2 = pool.insert("world".to_string());

        assert_eq!(pool.get(h1), Some(&"hello".to_string()));
        assert_eq!(pool.get(h2), Some(&"world".to_string()));
    }

    #[test]
    fn test_pool_remove_invalidates_handle() {
        let mut pool: ResourcePool<i32> = ResourcePool::new();
        let h = pool.insert(42);

        assert_eq!(pool.remove(h), Some(42));
        assert_eq!(pool.get(h), None);
        assert_eq!(pool.remove(h), None);
    }

    #[test]
    fn test_pool_reuses_slots() {
        let mut pool: ResourcePool<i32> = ResourcePool::new();
        let h1 = pool.insert(1);
        pool.remove(h1);

        let h2 = pool.insert(2);
        assert_eq!(h2.index, h1.index);
        assert_ne!(h2.generation, h1.generation);
        assert_eq!(pool.get(h1), None);
        assert_eq!(pool.get(h2), Some(&2));
    }

    #[test]
    fn test_pool_get_mut() {
        let mut pool: ResourcePool<String> = ResourcePool::new();
        let h = pool.insert("hello".to_string());

        if let Some(val) = pool.get_mut(h) {
            val.push_str(" world");
        }

        assert_eq!(pool.get(h), Some(&"hello world".to_string()));
    }

    #[test]
    #[should_panic(expected = "free list pointed to occupied slot")]
    fn test_pool_free_list_occupied_slot_panic() {
        let mut pool: ResourcePool<i32> = ResourcePool::new();
        let h1 = pool.insert(42);
        pool.remove(h1);

        // Tamper with the internal state to simulate a bug in the free list logic
        pool.entries[h1.index as usize] = PoolEntry::Occupied {
            value: 99,
            generation: h1.generation + 1,
        };

        // This insert should pull from the free list and encounter the illegally occupied slot
        pool.insert(100);
    }

    #[test]
    fn test_pool_default() {
        let pool: ResourcePool<i32> = ResourcePool::default();
        assert!(pool.entries.is_empty());
        assert!(pool.free_list.is_empty());
    }
}
