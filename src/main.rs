#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::fmt::Write;

mod driver;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut cli = driver::amd64::vga::Buf::default();
    write!(cli, "{}", info).unwrap();
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut cli = driver::amd64::vga::Buf::default();
    write!(cli, "Hello, vnix: {}", core::f32::consts::E).unwrap();

    panic!("some");

    loop {}
}
