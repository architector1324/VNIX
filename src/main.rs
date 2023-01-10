#![no_std]
#![no_main]
#![feature(abi_efiapi)]

pub mod vnix;
pub mod driver;

use core::fmt::Write;

use uefi::prelude::{entry, Handle, SystemTable, Boot, Status};
pub use uefi_services::println;

use vnix::vnix_entry;
use vnix::core::kern::Kern;


#[entry]
fn main(_image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    let cli = driver::amd64::term::Amd64Term::new(st);
    if let Err(ref err) = cli {
        println!("ERR loader: {:?}", err);
    }

    let mut cli = cli.unwrap();

    let kern = Kern::new(&mut cli);

    writeln!(kern.cli, "INFO vnix: kernel running on `amd64` platform").unwrap();

    if let Err(err) = vnix_entry(kern) {
        writeln!(cli, "ERR vnix: {:?}", err).unwrap();
    }
    cli.st.boot_services().stall(10_000_000);

    Status::SUCCESS
}
