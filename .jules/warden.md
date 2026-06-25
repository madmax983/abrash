2024-05-18 - [Vulnerable dependencies and out-of-bounds pointer write]
**Threat:** Vulnerable dependencies (memmap2, rand, imageproc) and potential out-of-bounds write in SendPtr::write due to reliance on caller checking length correctly in parallel execution paths.
**Defense:** Updated memmap2, rand, imageproc to secure versions and added index < length assertion inside SendPtr::write.
