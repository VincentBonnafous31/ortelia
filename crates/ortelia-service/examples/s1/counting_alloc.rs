//! A global allocator that counts allocations (measure M3), active only on demand.
//!
//! `GlobalAlloc` is an `unsafe` trait: the caller of `alloc` promises a valid layout, and the
//! implementation promises to return valid memory. This module only forwards every call,
//! unchanged, to the system allocator (`std::alloc::System`), and adds to two atomic counters
//! while counting is active. It is the only `unsafe` code of the benchmark, hence the local
//! exception to the workspace rule.
//!
//! Counting is off by default (an `AtomicBool`), so that the other cases are not slowed down
//! beyond one relaxed load per allocation.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

/// The allocator of the benchmark process.
pub struct CountingAlloc;

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

impl CountingAlloc {
    fn record(size: usize) {
        if ACTIVE.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(size as u64, Ordering::Relaxed);
        }
    }
}

// SAFETY: every method forwards its arguments unchanged to `System`, which upholds the
// contract of `GlobalAlloc`; the counters have no effect on the memory returned.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        Self::record(layout.size());
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        Self::record(layout.size());
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A reallocation counts as an allocation of the new size.
        Self::record(new_size);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// Resets the counters and starts counting.
pub fn start() {
    COUNT.store(0, Ordering::Relaxed);
    BYTES.store(0, Ordering::Relaxed);
    ACTIVE.store(true, Ordering::Relaxed);
}

/// Stops counting and returns the number of allocations and the bytes allocated.
pub fn stop() -> (u64, u64) {
    ACTIVE.store(false, Ordering::Relaxed);
    (COUNT.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed))
}

/// The counters, without changing whether counting is active.
#[cfg(test)]
pub fn counters() -> (u64, u64) {
    (COUNT.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test only: counting is global, two tests in parallel would disturb each other.
    #[test]
    fn counts_only_while_active() {
        start();
        let data: Vec<Vec<u8>> = (0..10).map(|i| vec![0; 100 + i]).collect();
        let (count, bytes) = stop();
        std::hint::black_box(&data);
        assert!(count >= 10, "{count}");
        assert!(bytes >= 1000, "{bytes}");

        let before = counters();
        let more: Vec<Vec<u8>> = (0..10).map(|i| vec![0; 100 + i]).collect();
        std::hint::black_box(&more);
        assert_eq!(counters(), before);
    }
}
