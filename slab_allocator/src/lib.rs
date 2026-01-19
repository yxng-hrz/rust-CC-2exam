#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::mem;

const SLAB_SIZE: usize = 4096;
const MAX_OBJECT_SIZE: usize = 512;

struct FreeNode {
    next: Option<NonNull<FreeNode>>,
}

pub struct Slab {
    memory: NonNull<u8>,
    free_list: Option<NonNull<FreeNode>>,
    object_size: usize,
    capacity: usize,
    allocated: usize,
}

#[cfg(test)]
mod tests {
    extern crate std;
}
