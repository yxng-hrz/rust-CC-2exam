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
            let ptr = core::alloc::alloc(layout);
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
            core::alloc::dealloc(self.memory.as_ptr(), layout);
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

#[cfg(test)]
mod tests {
    extern crate std;
}
