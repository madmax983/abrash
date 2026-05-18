import sys

content = open("crates/abrash-render/src/render_api/handles.rs", "r").read()

# Replace PoolEntry with Slot
content = content.replace("enum PoolEntry<T> {\n    Occupied { value: T, generation: Generation },\n    Vacant { generation: Generation },\n}", "struct Slot<T> {\n    value: Option<T>,\n    generation: Generation,\n}")

# Replace ResourcePool entries type
content = content.replace("entries: Vec<PoolEntry<T>>,", "entries: Vec<Slot<T>>,")

# Fix insert method
insert_old = """    pub fn insert(&mut self, value: T) -> Handle<T> {
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
    }"""
insert_new = """    pub fn insert(&mut self, value: T) -> Handle<T> {
        if let Some(index) = self.free_list.pop() {
            let entry = &mut self.entries[index as usize];
            if entry.value.is_some() {
                unreachable!("free list pointed to occupied slot");
            }
            let generation = entry.generation;
            entry.value = Some(value);
            Handle::new(index, generation)
        } else {
            let index = self.entries.len() as u32;
            let generation = 0;
            self.entries.push(Slot { value: Some(value), generation });
            Handle::new(index, generation)
        }
    }"""
content = content.replace(insert_old, insert_new)

# Fix get method
get_old = """    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.entries
            .get(handle.index as usize)
            .and_then(|entry| match entry {
                PoolEntry::Occupied { value, generation } if *generation == handle.generation => {
                    Some(value)
                }
                _ => None,
            })
    }"""
get_new = """    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.entries
            .get(handle.index as usize)
            .filter(|entry| entry.generation == handle.generation)
            .and_then(|entry| entry.value.as_ref())
    }"""
content = content.replace(get_old, get_new)

# Fix get_mut method
get_mut_old = """    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.entries
            .get_mut(handle.index as usize)
            .and_then(|entry| match entry {
                PoolEntry::Occupied { value, generation } if *generation == handle.generation => {
                    Some(value)
                }
                _ => None,
            })
    }"""
get_mut_new = """    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.entries
            .get_mut(handle.index as usize)
            .filter(|entry| entry.generation == handle.generation)
            .and_then(|entry| entry.value.as_mut())
    }"""
content = content.replace(get_mut_old, get_mut_new)

# Fix remove method
remove_old = """    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let entry = self.entries.get_mut(handle.index as usize)?;
        let old_gen = match entry {
            PoolEntry::Occupied { generation, .. } if *generation == handle.generation => {
                *generation
            }
            _ => return None,
        };

        let new_gen = if old_gen == Generation::MAX {
            Generation::MAX
        } else {
            old_gen + 1
        };
        let old_entry = std::mem::replace(
            entry,
            PoolEntry::Vacant {
                generation: new_gen,
            },
        );

        if old_gen != Generation::MAX {
            self.free_list.push(handle.index);
        }

        match old_entry {
            PoolEntry::Occupied { value, .. } => Some(value),
            PoolEntry::Vacant { .. } => unreachable!(),
        }
    }"""
remove_new = """    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let entry = self.entries.get_mut(handle.index as usize)?;

        if entry.generation != handle.generation || entry.value.is_none() {
            return None;
        }

        let old_gen = entry.generation;
        entry.generation = if old_gen == Generation::MAX {
            Generation::MAX
        } else {
            old_gen + 1
        };

        if old_gen != Generation::MAX {
            self.free_list.push(handle.index);
        }

        entry.value.take()
    }"""
content = content.replace(remove_old, remove_new)

# Fix tests
content = content.replace("PoolEntry::Occupied {\n            value: 99,\n            generation: h1.generation + 1,\n        }", "Slot {\n            value: Some(99),\n            generation: h1.generation + 1,\n        }")
content = content.replace("PoolEntry::Occupied {\n                value: \"Sensitive Data\".to_string(),\n                generation: Generation::MAX,\n            }", "Slot {\n                value: Some(\"Sensitive Data\".to_string()),\n                generation: Generation::MAX,\n            }")
content = content.replace("PoolEntry::Occupied {\n                value: 42,\n                generation: Generation::MAX,\n            }", "Slot {\n                value: Some(42),\n                generation: Generation::MAX,\n            }")

test_unreachable_old = """    #[test]
    #[should_panic(expected = "internal error: entered unreachable code")]
    fn test_pool_remove_unreachable_guard() {
        let old = PoolEntry::Vacant::<i32> { generation: 0 };
        match old {
            PoolEntry::Occupied { value, .. } => {
                let _ = value;
            }
            PoolEntry::Vacant { .. } => unreachable!(),
        }
    }"""
test_unreachable_new = """"""
content = content.replace(test_unreachable_old, test_unreachable_new)

open("crates/abrash-render/src/render_api/handles.rs", "w").write(content)
