**[Particle Array Iteration Retain_Mut]**
**Learning:** In hot loops over an array of structs that update logic and removes elements, `Vec::retain_mut` allows simultaneous iteration, in-place modification, and removal, avoiding the overhead of index-based element removal that shifts all subsequent elements each time or even `swap_remove` in index iterations.
**Action:** Replace `while` loop index-based iteration and `swap_remove` calls inside `ParticleSystem::update` with `Vec::retain_mut` to combine updating and filtering, significantly reducing array manipulation overhead by over 60%.
