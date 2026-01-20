#![no_std]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::mem;
use alloc::alloc::{alloc, dealloc};

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

impl Slab {
    pub fn new(object_size: usize) -> Option<Self> {
        if object_size == 0 || object_size > MAX_OBJECT_SIZE {
            return None;
        }

        let aligned_size = Self::align_size(object_size);
        let capacity = SLAB_SIZE / aligned_size;
        
        if capacity == 0 {
            return None;
        }

        let memory = Self::allocate_memory(SLAB_SIZE)?;
        let mut slab = Slab {
            memory,
            free_list: None,
            object_size: aligned_size,
            capacity,
            allocated: 0,
        };

        slab.init_free_list();
        Some(slab)
    }

    fn align_size(size: usize) -> usize {
        let align = mem::align_of::<FreeNode>().max(8);
        let node_size = mem::size_of::<FreeNode>();
        size.max(node_size).next_multiple_of(align)
    }

    fn allocate_memory(size: usize) -> Option<NonNull<u8>> {
        let layout = Layout::from_size_align(size, mem::align_of::<usize>()).ok()?;
        unsafe {
            let ptr = alloc(layout);
            NonNull::new(ptr)
        }
    }

    fn init_free_list(&mut self) {
        let base = self.memory.as_ptr() as usize;
        let mut prev: Option<NonNull<FreeNode>> = None;

        for i in (0..self.capacity).rev() {
            let offset = i * self.object_size;
            let node_ptr = (base + offset) as *mut FreeNode;
            
            unsafe {
                let node = &mut *node_ptr;
                node.next = prev;
                prev = NonNull::new(node_ptr);
            }
        }

        self.free_list = prev;
    }

    pub fn allocate(&mut self) -> Option<NonNull<u8>> {
        let node = self.free_list?;
        
        unsafe {
            self.free_list = (*node.as_ptr()).next;
        }
        
        self.allocated += 1;
        Some(node.cast())
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        let node_ptr = ptr.cast::<FreeNode>();
        
        unsafe {
            (*node_ptr.as_ptr()).next = self.free_list;
        }
        
        self.free_list = Some(node_ptr);
        self.allocated = self.allocated.saturating_sub(1);
    }

    pub fn is_full(&self) -> bool {
        self.allocated == self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.allocated == 0
    }

    pub fn contains(&self, ptr: NonNull<u8>) -> bool {
        let addr = ptr.as_ptr() as usize;
        let base = self.memory.as_ptr() as usize;
        let end = base + SLAB_SIZE;
        addr >= base && addr < end
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(SLAB_SIZE, mem::align_of::<usize>()).unwrap();
        unsafe {
            dealloc(self.memory.as_ptr(), layout);
        }
    }
}

pub struct SlabAllocator {
    slabs: [Option<Slab>; 16],
    object_size: usize,
}

impl SlabAllocator {
    pub const fn new(object_size: usize) -> Self {
        const NONE: Option<Slab> = None;
        SlabAllocator {
            slabs: [NONE; 16],
            object_size,
        }
    }

    pub fn allocate(&mut self) -> Option<NonNull<u8>> {
        for slab in self.slabs.iter_mut().flatten() {
            if !slab.is_full() {
                if let Some(ptr) = slab.allocate() {
                    return Some(ptr);
                }
            }
        }

        for slot in self.slabs.iter_mut() {
            if slot.is_none() {
                *slot = Slab::new(self.object_size);
                if let Some(slab) = slot {
                    return slab.allocate();
                }
            }
        }

        None
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        for slab in self.slabs.iter_mut().flatten() {
            if slab.contains(ptr) {
                slab.deallocate(ptr);
                return;
            }
        }
    }
}

pub struct SlabCache {
    small: SlabAllocator,
    medium: SlabAllocator,
    large: SlabAllocator,
}

impl SlabCache {
    pub const fn new() -> Self {
        SlabCache {
            small: SlabAllocator::new(64),
            medium: SlabAllocator::new(256),
            large: SlabAllocator::new(512),
        }
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size();
        
        if size <= 64 {
            self.small.allocate()
        } else if size <= 256 {
            self.medium.allocate()
        } else if size <= 512 {
            self.large.allocate()
        } else {
            None
        }
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = layout.size();
        
        if size <= 64 {
            self.small.deallocate(ptr);
        } else if size <= 256 {
            self.medium.deallocate(ptr);
        } else if size <= 512 {
            self.large.deallocate(ptr);
        }
    }
}

pub struct GlobalSlabAllocator;

unsafe impl GlobalAlloc for GlobalSlabAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        dealloc(ptr, layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use std::vec::Vec;

    #[test]
    fn test_slab_creation() {
        let slab = Slab::new(64);
        assert!(slab.is_some());
        let slab = slab.unwrap();
        assert_eq!(slab.object_size, 64);
        assert!(slab.capacity > 0);
        assert!(slab.is_empty());
    }

    #[test]
    fn test_slab_allocate_deallocate() {
        let mut slab = Slab::new(64).unwrap();
        let ptr = slab.allocate();
        assert!(ptr.is_some());
        assert!(!slab.is_empty());
        
        let ptr = ptr.unwrap();
        slab.deallocate(ptr);
        assert!(slab.is_empty());
    }

    #[test]
    fn test_slab_multiple_allocations() {
        let mut slab = Slab::new(64).unwrap();
        let mut ptrs = Vec::new();

        for _ in 0..10 {
            if let Some(ptr) = slab.allocate() {
                ptrs.push(ptr);
            }
        }

        assert_eq!(ptrs.len(), 10);
        assert_eq!(slab.allocated, 10);

        for ptr in ptrs {
            slab.deallocate(ptr);
        }

        assert!(slab.is_empty());
    }

    #[test]
    fn test_slab_full() {
        let mut slab = Slab::new(64).unwrap();
        let capacity = slab.capacity;
        let mut ptrs = Vec::new();

        for _ in 0..capacity {
            if let Some(ptr) = slab.allocate() {
                ptrs.push(ptr);
            }
        }

        assert!(slab.is_full());
        assert!(slab.allocate().is_none());

        slab.deallocate(ptrs[0]);
        assert!(!slab.is_full());
    }

    #[test]
    fn test_slab_contains() {
        let mut slab = Slab::new(64).unwrap();
        let ptr = slab.allocate().unwrap();
        assert!(slab.contains(ptr));
        
        let external = NonNull::new(0x1000 as *mut u8).unwrap();
        assert!(!slab.contains(external));
    }

    #[test]
    fn test_allocator_basic() {
        let mut allocator = SlabAllocator::new(64);
        let ptr = allocator.allocate();
        assert!(ptr.is_some());
        
        let ptr = ptr.unwrap();
        allocator.deallocate(ptr);
    }

    #[test]
    fn test_allocator_multiple_slabs() {
        let mut allocator = SlabAllocator::new(64);
        let mut ptrs = Vec::new();

        for _ in 0..200 {
            if let Some(ptr) = allocator.allocate() {
                ptrs.push(ptr);
            }
        }

        assert!(ptrs.len() >= 100);

        for ptr in ptrs {
            allocator.deallocate(ptr);
        }
    }

    #[test]
    fn test_cache_small_allocation() {
        let mut cache = SlabCache::new();
        let layout = Layout::from_size_align(32, 8).unwrap();
        let ptr = cache.allocate(layout);
        assert!(ptr.is_some());
        
        let ptr = ptr.unwrap();
        cache.deallocate(ptr, layout);
    }

    #[test]
    fn test_cache_medium_allocation() {
        let mut cache = SlabCache::new();
        let layout = Layout::from_size_align(128, 8).unwrap();
        let ptr = cache.allocate(layout);
        assert!(ptr.is_some());
        
        let ptr = ptr.unwrap();
        cache.deallocate(ptr, layout);
    }

    #[test]
    fn test_cache_large_allocation() {
        let mut cache = SlabCache::new();
        let layout = Layout::from_size_align(400, 8).unwrap();
        let ptr = cache.allocate(layout);
        assert!(ptr.is_some());
        
        let ptr = ptr.unwrap();
        cache.deallocate(ptr, layout);
    }

    #[test]
    fn test_cache_oversized() {
        let mut cache = SlabCache::new();
        let layout = Layout::from_size_align(1024, 8).unwrap();
        let ptr = cache.allocate(layout);
        assert!(ptr.is_none());
    }

    #[test]
    fn test_zero_size() {
        let slab = Slab::new(0);
        assert!(slab.is_none());
    }

    #[test]
    fn test_large_object() {
        let slab = Slab::new(MAX_OBJECT_SIZE + 1);
        assert!(slab.is_none());
    }

    #[test]
    fn test_alignment() {
        let mut slab = Slab::new(17).unwrap();
        let ptr = slab.allocate().unwrap();
        let addr = ptr.as_ptr() as usize;
        assert_eq!(addr % 8, 0);
    }

    #[test]
    fn test_reuse_freed_memory() {
        let mut slab = Slab::new(64).unwrap();
        let ptr1 = slab.allocate().unwrap();
        let addr1 = ptr1.as_ptr() as usize;
        
        slab.deallocate(ptr1);
        
        let ptr2 = slab.allocate().unwrap();
        let addr2 = ptr2.as_ptr() as usize;
        
        assert_eq!(addr1, addr2);
    }
}
