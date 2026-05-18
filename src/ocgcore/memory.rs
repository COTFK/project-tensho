/// WASM memory management tools.
use crate::ocgcore::OCGCore;
use std::ops::Add;
use wasm_bindgen::JsValue;

/// A safe wrapper around an address in WASM memory.
/// Preferable to accidentally feeding the FFI
/// a random [`u32`] that doesn't belong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorePointer(pub(super) u32);

impl CorePointer {
    pub const fn new(address: u32) -> Self {
        Self(address)
    }

    pub fn offset_by(&self, offset: usize) -> Self {
        *self + offset
    }
}

impl From<CorePointer> for usize {
    fn from(ptr: CorePointer) -> Self {
        ptr.0 as Self
    }
}

impl From<CorePointer> for u32 {
    fn from(ptr: CorePointer) -> Self {
        ptr.0
    }
}

impl From<CorePointer> for JsValue {
    fn from(ptr: CorePointer) -> Self {
        Self::from_f64(f64::from(ptr.0))
    }
}

impl Add<usize> for CorePointer {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs as u32)
    }
}

/// Manually allocated memory in WASM.
///
/// [`WASMMemoryAllocation::new()`] sets up the _malloc call,
/// and the [`Drop`] implementation handles the _free call automatically.
///
#[derive(Debug, Clone)]
pub struct CoreMemoryAllocation<'a> {
    core: &'a OCGCore,
    pointer: CorePointer,
}

impl<'a> CoreMemoryAllocation<'a> {
    pub const fn new(core: &'a OCGCore, pointer: CorePointer) -> Self {
        Self { core, pointer }
    }

    pub const fn get_pointer(&self) -> CorePointer {
        self.pointer
    }
}

impl Drop for CoreMemoryAllocation<'_> {
    /// Free the allocated memory.
    fn drop(&mut self) {
        self.core.instance.free(self.pointer.0);
    }
}
