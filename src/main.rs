#![no_std]
#![no_main]
#![feature(abi_efiapi)]

extern crate alloc;

pub mod vnix;
pub mod driver;

use core::fmt::Write;

use driver::Disp;
use driver::Rnd;
use uefi::prelude::{entry, Handle, SystemTable, Boot, Status};
pub use uefi_services::println;

use vnix::vnix_entry;
use vnix::core::kern::Kern;


#[entry]
fn main(_image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    st.stdout().clear().unwrap();

    // disable watchdog timer to avoid reboot after 5 minutes
    st.boot_services().set_watchdog_timer(0, 0xffff + 1, None).unwrap();

    unsafe {
        // load drivers

        // cli
        let cli = driver::uefi::UefiCLI::new(st.unsafe_clone());
        if cli.is_err() {
            println!("ERR loader:cli: not available");
        }
    
        let mut cli = cli.unwrap();
    
        // disp
        let mut disp = driver::uefi::UefiDisp::new(st.unsafe_clone());
        if disp.is_err() {
            println!("ERR loader:disp: not available");
            println!("WARN loader:disp: using stub driver");
        }

        let mut disp_stub = driver::stub::StubDisp;
    
        // time
        let time = driver::uefi::UefiTime::new(st.unsafe_clone());
        if time.is_err() {
            println!("ERR loader:time: not available");
        }
    
        let mut time = time.unwrap();

        // rnd
        let mut rnd = driver::uefi::UefiRnd::new(st.unsafe_clone());
        if rnd.is_err() {
            println!("ERR loader:rnd not available");
            println!("WARN loader:rnd: using pseudo random generator");
        }

        let mut prng = driver::stub::PRng;

        // load kernel
        let kern = Kern::new(
            &mut cli,
            disp.as_mut().map(|p| p as &mut dyn Disp).unwrap_or(&mut disp_stub),
            &mut time,
            rnd.as_mut().map(|p| p as &mut dyn Rnd).unwrap_or(&mut prng)
        );

        writeln!(kern.cli, "INFO vnix: kernel running on `uefi` platform").unwrap();

        // run
        if let Err(err) = vnix_entry(kern) {
            writeln!(cli, "ERR vnix: {:?}", err).unwrap();
        }
    }

    st.boot_services().stall(10_000_000);

    Status::SUCCESS
}
