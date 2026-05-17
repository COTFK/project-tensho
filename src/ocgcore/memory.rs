/// WASM memory management tools.
use crate::ocgcore::OCGCore;
use std::ops::Add;
use wasm_bindgen::JsValue;

/// A safe wrapper around an address in WASM memory.
/// Preferable to accidentally feeding the FFI
/// a random [`u32`] that doesn't belong.
#[derive(Debug, Clone, Copy)]
pub struct CorePointer(pub(super) u32);

impl CorePointer {
    pub fn new(address: u32) -> Self {
        Self(address)
    }

    pub fn offset_by(&self, offset: usize) -> CorePointer {
        *self + offset
    }
}

impl From<CorePointer> for usize {
    fn from(ptr: CorePointer) -> Self {
        ptr.0 as usize
    }
}

impl From<CorePointer> for u32 {
    fn from(ptr: CorePointer) -> Self {
        ptr.0
    }
}

impl From<CorePointer> for JsValue {
    fn from(ptr: CorePointer) -> Self {
        JsValue::from_f64(ptr.0 as f64)
    }
}

impl Add<usize> for CorePointer {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        CorePointer(self.0 + rhs as u32)
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
    pub fn new(core: &'a OCGCore, pointer: CorePointer) -> Self {
        Self { core, pointer }
    }

    pub fn get_pointer(&self) -> CorePointer {
        self.pointer
    }
}

impl Drop for CoreMemoryAllocation<'_> {
    /// Free the allocated memory.
    fn drop(&mut self) {
        self.core.instance.free(self.pointer.0);
    }
}
