#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(test_runner)]

use alloc::{boxed::Box, vec::Vec};
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use os_rust_demo::test_runner;
use os_rust_demo::{
    allocator::{self, HEAP_SIZE},
    init,
    memory::{self, BootInfoFrameAllocator},
    test_panic_handler,
};
use x86_64::VirtAddr;

extern crate alloc;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(4);
    let heap_value_2 = Box::new(5);
    assert_eq!(*heap_value_1, 4);
    assert_eq!(*heap_value_2, 5);
}

#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}
