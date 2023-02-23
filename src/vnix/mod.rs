pub mod core;
pub mod serv;
pub mod utils;

use alloc::vec;

use crate::driver::CLIErr;

use self::core::task::TaskLoop;
use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};
use self::core::serv::{Serv, ServKind};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    // register service
    let services = [
        ("io.term", ServKind::IOTerm),
        ("io.store", ServKind::IOStore),
        ("etc.chrono", ServKind::EtcChrono),
        ("etc.fsm", ServKind::EtcFSM),
        ("gfx.2d", ServKind::GFX2D),
        ("math.int", ServKind::MathInt),
        ("sys.task", ServKind::SysTask),
        ("sys.usr", ServKind::SysUsr),
        ("test.dumb", ServKind::TestDumb),
    ];

    for (name, kind) in services {
        let serv = Serv::new(name, kind);
        kern.reg_serv(serv)?;

        writeln!(kern.drv.cli, "INFO vnix:kern: service `{}` registered", name).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    }

    // register user
    let _super = Usr::new("super", &mut kern)?.0;
    kern.reg_usr(_super.clone())?;

    writeln!(kern.drv.cli, "INFO vnix:kern: user `{}` registered", _super).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    // test
    let s = "{term:[cls (get.res @res.cli) (get.res.lst.gfx @res.gfx) (say @res) nl]}";
    let msg = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    let task = TaskLoop::Queue(vec![(msg, "io.term".into())]);

    // // run
    // let path = Unit::parse("@task.hello.gfx".chars()).map_err(|e| KernErr::ParseErr(e))?.0;
    // let msg = kern.ram_store.load(path).ok_or(KernErr::DbLoadFault)?;

    // let task = TaskLoop::Queue(vec![(msg, "sys.task".into())]);

    kern.reg_task(&_super.name, "init.load", task)?;
    kern.run()
}
