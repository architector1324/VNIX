#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(iter_array_chunks)]
#![feature(array_chunks)]

extern crate alloc;

pub mod vnix;
pub mod driver;

use core::fmt::Write;

use alloc::boxed::Box;
use alloc::string::String;
use driver::Disp;
use driver::MemSizeUnits;
use driver::Rnd;
use uefi::prelude::cstr16;
use uefi::prelude::{entry, Handle, SystemTable, Boot, Status};
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
pub use uefi_services::println;

use vnix::vnix_entry;
use vnix::core::kern::Kern;

use crate::vnix::core::unit::Unit;


fn load_store(image: Handle, st: SystemTable<Boot>) -> Option<Unit> {
    let mut fs_hlr = st.boot_services().get_image_file_system(image).ok()?;
    let mut fs = fs_hlr.open_volume().ok()?;

    let mut store_file = fs.open(cstr16!("vnix.store"), FileMode::Read, FileAttribute::VALID_ATTR).ok()?.into_regular_file()?;
    let mut store_buf = Box::new([0; 256 * 1024 * 1024]);

    store_file.read(store_buf.as_mut()).ok()?;

    let store_s = String::from_utf8_lossy(store_buf.as_slice());
    Some(Unit::parse(store_s.chars()).ok()?.0)
}


#[entry]
fn main(image: Handle, mut st: SystemTable<Boot>) -> Status {
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
            println!("ERR loader: rnd not available");
            println!("WARN loader:rnd: using pseudo random generator");
        }

        let mut prng = driver::stub::PRng;

        // mem
        let mem = driver::uefi::UefiMem::new(st.unsafe_clone());
        if mem.is_err() {
            println!("ERR loader:mem: not available");
        }

        let mut mem = mem.unwrap();

        // load kernel
        let mut kern = Kern::new(
            &mut cli,
            disp.as_mut().map(|p| p as &mut dyn Disp).unwrap_or(&mut disp_stub),
            &mut time,
            rnd.as_mut().map(|p| p as &mut dyn Rnd).unwrap_or(&mut prng),
            &mut mem
        );

        // load store
        writeln!(kern.cli, "INFO vnix: load `super` storage").unwrap();

        if let Some(store) = load_store(image, st.unsafe_clone()) {
            kern.ram_store.data = store;
        } else {
            println!("ERR loader: store not available");
            return Status::ABORTED;
        }

        // run
        writeln!(kern.cli, "INFO vnix: kernel running on `uefi` platform").unwrap();
        writeln!(kern.cli, "INFO vnix:kern: {}mb. free memory", kern.mem.free(MemSizeUnits::Mega).unwrap()).unwrap();

        if let Err(err) = vnix_entry(kern) {
            writeln!(cli, "ERR vnix: {:?}", err).unwrap();
        }
    }

    st.boot_services().stall(10_000_000);

    Status::SUCCESS
}
