#![no_std]
#![no_main]

use core::panic::PanicInfo;

use os_rust_demo::{exit_qemu, serial_println, QemuExitCode};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {

        serial_println!("[test did not panic]");
        exit_qemu(QemuExitCode::Failed);
    loop {}
}

