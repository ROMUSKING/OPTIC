//! optic-runtime — the tiny, zero-cost runtime support for the narrow v0 Rust backend.
//!
//! From the book (ch. 5, 8, 11):
//! - `Cursor<S>` is the operational bridge: { arena: &mut S, id: usize }.
//! - All hot paths use direct indexing via the cursor id.
//! - No hidden semantics, no reimplementation of language rules at runtime.
//! - SoA layouts are plain struct { col: Vec<T>, ... } in the generated code.

#![allow(dead_code)]

use std::fmt;

/// The Cursor that appears in every generated loop (book ch. 5/8/11).
/// `arena` is the base for the costate (e.g. &mut Entities).
/// `id` is the induction variable / entity index for this iteration.
///
/// Note: intentionally contains &mut, therefore not Clone/Copy.
pub struct Cursor<'a, S> {
    pub arena: &'a mut S,
    pub id: usize,
}

impl<'a, S> Cursor<'a, S> {
    #[inline(always)]
    pub fn new(arena: &'a mut S, id: usize) -> Self {
        Cursor { arena, id }
    }

    /// Convenience for examples that want a read-only view (rare in v0 hot paths).
    pub fn id(&self) -> usize {
        self.id
    }
}

/// Optional debug helper (only used in non-hot paths or test builds).
/// The book wants the generated code to be obviously direct indexing.
#[inline]
pub fn debug_bounds_check(len: usize, id: usize, field: &str) {
    if id >= len {
        panic!(
            "optic runtime bounds error on field {}: id={} >= len={}",
            field, id, len
        );
    }
}

/// A tiny helper sometimes useful in generated drivers for examples.
/// Real programs own their SoA structs directly.
pub fn len_of<T>(v: &[T]) -> usize {
    v.len()
}

/// Runtime hooks for profile/replay (M8+ observability; v0 stubs for CLI/runtime surface).
/// Grade-controlled erasure / real impl deferred (see docs/observability-v0.md).
#[inline]
pub fn profile(_label: &str) {}

#[inline]
pub fn replay(_checkpoint: &str) {}

impl<S: fmt::Debug> fmt::Debug for Cursor<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cursor")
            .field("id", &self.id)
            .field("arena", &"<&mut S>")
            .finish()
    }
}
