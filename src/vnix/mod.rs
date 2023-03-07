pub mod core;
pub mod serv;
pub mod utils;

use alloc::boxed::Box;
use alloc::vec;

use crate::driver::{CLIErr, DrvErr};

use self::core::task::TaskLoop;
use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};
use self::core::serv::{Serv, ServHlr};
use self::serv::{/*io, math, gfx, etc, sys,*/ time, test};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    // register service
    let services = [
        // ("io.term", Box::new(io::term::Term::default()) as Box<dyn ServHlr>),
        // ("io.store", Box::new(io::store::Store::default()) as Box<dyn ServHlr>),
        // ("etc.fsm", Box::new(etc::fsm::FSM::default()) as Box<dyn ServHlr>),
        (time::chrono::SERV_PATH, time::chrono::SERV_HELP, Box::new(time::chrono::chrono_hlr) as Box<ServHlr>),
        // ("gfx.2d", Box::new(gfx::GFX2D::default()) as Box<dyn ServHlr>),
        // ("math.calc", Box::new(math::Calc::default()) as Box<dyn ServHlr>),
        // ("sys.task", Box::new(sys::task::Task::default()) as Box<dyn ServHlr>),
        // ("sys.usr", Box::new(sys::usr::User::default()) as Box<dyn ServHlr>),
        // ("sys.hw", Box::new(sys::hw::HW::default()) as Box<dyn ServHlr>),
        (test::dumb::SERV_PATH, test::dumb::SERV_HELP, Box::new(test::dumb::dumb_hlr) as Box<ServHlr>),
    ];

    for (name, help, hlr) in services {
        let serv = Serv::new(name, help, hlr);
        kern.reg_serv(serv)?;

        writeln!(kern.drv.cli, "INFO vnix:kern: service `{}` registered", name).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
    }

    // register user
    let _super = Usr::new("super", &mut kern)?.0;
    kern.reg_usr(_super.clone())?;

    writeln!(kern.drv.cli, "INFO vnix:kern: user `{}` registered", _super).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;

    // test
    let s0 = "{wait.ms:{msg:2000}@test.dumb}";
    let test_msg0 = Unit::parse(s0.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    let s1 = "a";
    let test_msg1 = Unit::parse(s1.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    let task = TaskLoop::Queue(vec![
        (test_msg0, "time.chrono".into()),
        (test_msg1, "test.dumb".into())
    ]);

    // run
    // let path = Unit::parse("@task.init.gfx.cli".chars()).map_err(|e| KernErr::ParseErr(e))?.0;
    // let msg = kern.ram_store.load(path).ok_or(KernErr::DbLoadFault)?;

    // let task = TaskLoop::Queue(vec![(test_msg, "io.term".into()), (msg, "sys.task".into())]);
    kern.reg_task(&_super.name, "init.load", task)?;

    kern.run()
}
