**[Eliminating `.clone()` in Generics bounded only by `Clone`]**
**Learning:** When trying to eliminate `.clone()` calls on generic wrapper structs (like `Sample<T>`), if the generic parameter `T` only implements `Clone` and not `Copy`, removing `.clone()` will cause borrow checker 'move' errors. Workarounds like `std::mem::take` require adding a `Default` bound, which constitutes a breaking API change.
**Action:** Accept the `.clone()` in these specific generic contexts or verify if the struct parameter can safely be bounded by `Copy` without causing broader API breakages.
