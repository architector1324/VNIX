#![no_std]
#![no_main]

#![feature(array_chunks)]
#![feature(drain_filter)]
#![feature(iter_array_chunks)]
#![feature(generators, generator_trait)]
#![feature(type_alias_impl_trait)]
#![feature(arc_unwrap_or_clone)]
#![feature(result_option_inspect)]
#![feature(iterator_try_reduce)]
#![feature(iterator_try_collect)]
#![feature(associated_type_defaults)]

extern crate alloc;

mod vnix;
mod driver;
mod content;

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use core::fmt::Write;

use uefi::prelude::cstr16;
pub use uefi_services::println;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::file::FileAttribute;
use uefi::prelude::{entry, Handle, SystemTable, Boot, Status};

use driver::Disp;
use driver::MemSizeUnits;
use driver::Rnd;

use vnix::vnix_entry;
use vnix::core::kern::Kern;
use vnix::core::kern::KernDrv;
use vnix::serv::io::term::Mode;
use vnix::serv::io::term::base;

use crate::vnix::core::unit::{Unit, UnitParse};


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
    
        let cli = cli.unwrap();
    
        // disp
        let disp = driver::uefi::UefiDisp::new(st.unsafe_clone());
        if disp.is_err() {
            println!("WARN loader:disp: not available, using stub driver");
        }

        let disp_stub = driver::stub::StubDisp;

        // time
        let time = driver::uefi::UefiTime::new(st.unsafe_clone());
        if time.is_err() {
            println!("ERR loader:time: not available");
        }

        let time = time.unwrap();

        // rnd
        let rnd = driver::uefi::UefiRnd::new(st.unsafe_clone());
        if rnd.is_err() {
            println!("WARN loader:rnd: not available, using pseudo random generator");
        }

        let prng = driver::stub::PRng([1; 32]);

        // mem
        let mem = driver::uefi::UefiMem::new(st.unsafe_clone());
        if mem.is_err() {
            println!("ERR loader:mem: not available");
        }

        let mem = mem.unwrap();

        // kernel console
        let term = Rc::new(Mutex::new(base::Term::new(&content::SYS_FONT)));

        if disp.is_err() {
            term.lock().mode = Mode::Text;
        }

        // drivers
        let driver = KernDrv::new(
            Box::new(cli),
            disp.map(|p| Box::new(p) as Box<dyn Disp>).unwrap_or(Box::new(disp_stub) as Box<dyn Disp>),
            Box::new(time),
            rnd.map(|p| Box::new(p) as Box<dyn Rnd>).unwrap_or(Box::new(prng) as Box<dyn Rnd>),
            Box::new(mem)
        );

        // load kernel
        let mut kern = Kern::new(driver, term);

        // load store
        writeln!(kern, "INFO vnix: load `super` storage").unwrap();

        if let Some(store) = load_store(image, st.unsafe_clone()) {
            kern.ram_store.data = store;
        } else {
            println!("ERR loader: store not available");
            return Status::ABORTED;
        }

        // run
        writeln!(kern, "INFO vnix: kernel running on `uefi` platform").unwrap();

        let mode = kern.term.lock().mode.clone();
        writeln!(kern, "INFO vnix:kern: `{}` console mode", mode).unwrap();
        writeln!(kern, "INFO vnix:kern: {}mb. free memory", kern.drv.mem.free(MemSizeUnits::Mega).unwrap()).unwrap();

        if let Err(err) = vnix_entry(kern) {
            println!("ERR vnix: {:?}", err);
        }
    }

    st.boot_services().stall(10_000_000);

    Status::SUCCESS
}
