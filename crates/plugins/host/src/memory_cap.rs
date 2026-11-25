//! The memory cap of a plugin's store: wasmtime's `StoreLimits`, and a
//! note of whether the plugin hit the cap, so the failure can say so.

use wasmtime::{ResourceLimiter, StoreLimits, StoreLimitsBuilder};

/// The memory a plugin may use (spec "Engine limits").
pub const MEMORY_CAP: usize = 64 * 1024 * 1024;

/// How many instances one store may hold: one component, a few inner
/// instances.
const INSTANCES: usize = 4;

/// `StoreLimits` of [`MEMORY_CAP`] and four instances, trapping when
/// memory can not grow, that remember whether the cap was hit.
pub(crate) struct MemoryCap {
    limits: StoreLimits,
    hit: bool,
}

impl MemoryCap {
    /// Whether the plugin asked for more memory than the cap.
    pub(crate) fn hit(&self) -> bool {
        self.hit
    }

    pub(crate) fn new() -> Self {
        let limits = StoreLimitsBuilder::new()
            .memory_size(MEMORY_CAP)
            .instances(INSTANCES)
            .trap_on_grow_failure(true)
            .build();

        Self { limits, hit: false }
    }
}

impl ResourceLimiter for MemoryCap {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > MEMORY_CAP {
            self.hit = true;
        }

        self.limits.memory_growing(current, desired, maximum)
    }

    fn memory_grow_failed(
        &mut self,
        error: wasmtime::Error,
    ) -> wasmtime::Result<()> {
        self.limits.memory_grow_failed(error)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.limits.table_growing(current, desired, maximum)
    }

    fn table_grow_failed(
        &mut self,
        error: wasmtime::Error,
    ) -> wasmtime::Result<()> {
        self.limits.table_grow_failed(error)
    }

    fn instances(&self) -> usize {
        self.limits.instances()
    }

    fn tables(&self) -> usize {
        self.limits.tables()
    }

    fn memories(&self) -> usize {
        self.limits.memories()
    }
}
