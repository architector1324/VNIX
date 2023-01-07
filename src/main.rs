#![no_std]
#![no_main]
#![feature(abi_efiapi)]


pub mod vnix;
pub mod driver;

use core::format_args;

use driver::CLI;
use uefi::prelude::{entry, Handle, SystemTable, Boot, Status};
use vnix::{vnix_entry, kern::Kern};


#[entry]
fn main(fb: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    let mut cli = driver::amd64::cli::Amd64CLI {
        st: st
    };

    let kern = Kern::new(&mut cli);

    if let Err(err) = vnix_entry(kern) {
        cli.println(format_args!("ERR vnix: {:?}", err));
        cli.st.boot_services().stall(10_000_000);
    }

    Status::SUCCESS
}
