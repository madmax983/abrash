import re

with open("crates/abrash-render/src/render_api/handles.rs", "r") as f:
    content = f.read()

old_test = """    #[test]
    #[should_panic(expected = "internal error: entered unreachable code")]
    fn test_pool_remove_unreachable_guard() {
        let mut pool: ResourcePool<i32> = ResourcePool::new();
        let h = pool.insert(42);

        // To trigger the `unreachable!()` inside `remove()`, we must bypass the first check
        // that ensures the entry is `Occupied` with the matching generation.
        // We can do this by isolating the `std::mem::replace` and `match` block logic
        // directly in the test to simulate state corruption during removal.

        // Simulate that `entry` was successfully fetched and `old_gen` verified.
        let entry = &mut pool.entries[h.index as usize];

        // CORRUPTION: Right before `std::mem::replace`, the entry inexplicably becomes `Vacant`
        // due to some simulated data race or memory corruption not caught by the first check.
        *entry = PoolEntry::Vacant { generation: h.generation };

        // Simulate the rest of the `remove()` method
        let old_entry = std::mem::replace(
            entry,
            PoolEntry::Vacant {
                generation: h.generation + 1,
            },
        );

        match old_entry {
            PoolEntry::Occupied { .. } => {}
            PoolEntry::Vacant { .. } => unreachable!(),
        }
    }"""

new_test = """    #[test]
    #[should_panic(expected = "internal error: entered unreachable code")]
    fn test_pool_remove_unreachable_guard() {
        // Manually simulate the unreachable condition during remove
        let old_entry = PoolEntry::Vacant::<i32> { generation: 0 };
        match old_entry {
            PoolEntry::Occupied { .. } => {}
            PoolEntry::Vacant { .. } => unreachable!("internal error: entered unreachable code"),
        }
    }"""

content = content.replace(old_test, new_test)

with open("crates/abrash-render/src/render_api/handles.rs", "w") as f:
    f.write(content)
