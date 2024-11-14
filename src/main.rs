#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os_rust_demo::test_runner)]
#![reexport_test_harness_main = "test_main"]

use alloc::vec;
use alloc::{boxed::Box, rc::Rc, vec::Vec};
use bootloader::{entry_point, BootInfo};
use os_rust_demo::task::executor::Executor;
use core::panic::PanicInfo;
use os_rust_demo::allocator::init_heap;
use os_rust_demo::task::simple_executor::SimpleExecutor;
use os_rust_demo::task::{keyboard, Task};
use os_rust_demo::{
    allocator, hlt_loop, init,
    memory::{self, translate_addr, BootInfoFrameAllocator},
    println,
};
use x86_64::{
    structures::paging::{Page, Translate},
    VirtAddr,
};

entry_point!(kernel_main);

extern crate alloc;

pub extern "C" fn _main_(boot_info: &'static BootInfo) -> ! {
    println!("Hello World{}", "!");
    init();

    use x86_64::registers::control::Cr3;

    let (level_4_page_table, _) = Cr3::read();
    println!(
        "Level 4 page table at: {:?}",
        level_4_page_table.start_address()
    );

    #[cfg(test)]
    test_main();

    println!("It did not crash!");

    hlt_loop();
}

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World!");

    init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();

}
async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}


/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}
