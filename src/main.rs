#![feature(array_chunks)]
#![feature(extract_if)]
#![feature(iter_array_chunks)]
#![feature(type_alias_impl_trait)]
#![feature(iterator_try_reduce)]
#![feature(iterator_try_collect)]
#![feature(associated_type_defaults)]
#![feature(slice_as_chunks)]
#![feature(async_closure)]

extern crate alloc;

mod vnix;
mod driver;
mod content;

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use core::fmt::Write;

use std::io::Read;
use std::{thread, fs::File, time::Duration};

use vnix::vnix_entry;
use vnix::core::kern::Kern;
use vnix::core::kern::KernDrv;
use vnix::serv::io::term::Mode;
use vnix::serv::io::term::base;

use crate::vnix::core::driver::CLI;
use crate::vnix::core::driver::Disp;
use crate::vnix::core::driver::MemSizeUnits;
use crate::vnix::core::unit::{Unit, UnitParse};


fn load_store() -> Option<Unit> {
    let mut store_file = File::open("vnix.store").ok()?;

    let mut store_s = String::new();
    store_file.read_to_string(&mut store_s).ok()?;

    Some(Unit::parse(store_s.chars()).ok()?.0)
}

fn main() {
    // load drivers

    // cli
    let cli = driver::linux::LinuxCLI::new();
    if cli.is_err() {
        println!("ERR loader:cli: not available");
    }

    let mut cli = cli.unwrap();
    cli.clear().unwrap();

    // disp
    let disp = driver::linux::LinuxDisp::new();

    if disp.is_err() {
        println!("WARN loader:disp: not available, using stub driver");
    }

    let disp_stub = driver::stub::StubDisp;

    // others
    let time = driver::linux::LinuxTime::new();
    let rnd = driver::linux::LinuxRnd;
    let mem = driver::linux::LinuxMem;

    // kernel console
    let term = Rc::new(Mutex::new(base::Term::new(&content::SYS_FONT)));

    // if disp.is_err() {
        term.lock().mode = Mode::Text;
    // }

    // drivers
    let driver = KernDrv::new(
        Box::new(cli),
        // disp.map(|p| Box::new(p) as Box<dyn Disp>).unwrap_or(Box::new(disp_stub) as Box<dyn Disp>),
        Box::new(disp_stub),
        Box::new(time),
        // rnd.map(|p| Box::new(p) as Box<dyn Rnd>).unwrap_or(Box::new(prng) as Box<dyn Rnd>),
        Box::new(rnd),
        Box::new(mem)
    );

    // load kernel
    let mut kern = Kern::new(driver, term);

    // load store
    writeln!(kern, "INFO vnix: load `vnix.store` storage").unwrap();

    if let Some(store) = load_store() {
        kern.ram_store.data = kern.new_unit(store);
    } else {
        println!("ERR loader: store not available");
        return;
    }

    // run
    kern.drv.time.start().unwrap();
    writeln!(kern, "INFO vnix: kernel running on `linux` platform").unwrap();

    let mode = kern.term.lock().mode.clone();
    writeln!(kern, "INFO vnix:kern: `{}` console mode", mode).unwrap();
    writeln!(kern, "INFO vnix:kern: {}mb. free memory", kern.drv.mem.free(MemSizeUnits::Mega).unwrap()).unwrap();

    if let Err(err) = vnix_entry(kern) {
        println!("ERR vnix: {:?}", err);
    }

    thread::sleep(Duration::from_secs(10));
}
