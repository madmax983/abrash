## [Reduction]
**Bloat:** Clippy warnings for `clippy::missing_const_for_fn`, `clippy::unreadable_literal`, `clippy::unnecessary_wraps`, `clippy::doc_markdown`, `clippy::must_use_candidate`, `clippy::redundant_closure_for_method_calls`, `clippy::non_send_fields_in_send_ty`, `clippy::float_cmp`, `clippy::field_reassign_with_default`, `clippy::option_if_let_else`, `clippy::needless_pass_by_value`, and `clippy::ptr_as_ptr`.
**Cut:** Fixed all the warnings using scripts, updated numeric literals, made functions const, added must_use, simplified if-let with map_or_else, and properly allowed warnings locally.
**Saved:** Hundreds of lines of potential cognitive load due to messy and inconsistent formatting.

