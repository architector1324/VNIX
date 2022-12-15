#![no_std]
#![no_main]

use core::panic::PanicInfo;

pub mod driver;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    driver::vga::puts(b"Hello, vnix!");
    loop {}
}
