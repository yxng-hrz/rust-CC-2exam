#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::mem;

const SLAB_SIZE: usize = 4096;
const MAX_OBJECT_SIZE: usize = 512;

struct FreeNode {
    next: Option<NonNull<FreeNode>>,
}

#[cfg(test)]
mod tests {
    extern crate std;
}
