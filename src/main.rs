#![no_std]
#![no_main]
#![feature(abi_efiapi)]

extern crate alloc;

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

    unsafe {
        // load drivers
        let cli = driver::amd64::Amd64CLI::new(st.unsafe_clone());
        if let Err(ref err) = cli {
            println!("ERR loader: {:?}", err);
        }
    
        let mut cli = cli.unwrap();
    
        let disp = driver::amd64::Amd64Disp::new(st.unsafe_clone());
        if let Err(ref err) = disp {
            println!("ERR loader: {:?}", err);
        }
    
        let mut disp = disp.unwrap();
    
        let time = driver::amd64::Amd64Time::new(st.unsafe_clone());
        if let Err(ref err) = time {
            println!("ERR loader: {:?}", err);
        }
    
        let mut time = time.unwrap();
    
        // load kernel
        let kern = Kern::new(&mut cli, &mut disp, &mut time);
    
        writeln!(kern.cli, "INFO vnix: kernel running on `amd64` platform").unwrap();
    
        // run
        if let Err(err) = vnix_entry(kern) {
            writeln!(cli, "ERR vnix: {:?}", err).unwrap();
        }
    }


    st.boot_services().stall(10_000_000);

    Status::SUCCESS
}
