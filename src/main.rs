#![no_std]
#![no_main]
#![feature(abi_efiapi)]


pub mod vnix;
pub mod driver;

use uefi::prelude::{entry, Handle, SystemTable, Boot, Status};
use vnix::vnix_entry;


#[entry]
fn main(fb: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    let mut cli = driver::amd64::cli::Amd64CLI{
        st: st
    };

    vnix_entry(&mut cli);

    // st.boot_services().stall(10_000_000);
    Status::SUCCESS
}
