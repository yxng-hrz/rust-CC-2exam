#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::mem;

#[cfg(test)]
mod tests {
    extern crate std;
}
