#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use uefi::prelude::{entry, Handle, SystemTable, Boot, Status};
use uefi_services::{println, print};

#[entry]
fn main(fb: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    uefi_services::system_table();
    println!("Hello, vnix!");

    let input = st.stdin();

    loop {
        if let Some(key) = input.read_key().unwrap() {
            print!("{:?} ", key);
        }
    }

    // st.boot_services().stall(10_000_000);
    // Status::SUCCESS
}
