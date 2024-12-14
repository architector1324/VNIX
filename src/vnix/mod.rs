pub mod core;
pub mod serv;
pub mod utils;

use ::core::fmt::Write;
use ::core::writeln;

use futures::executor::block_on;

use crate::vnix::core::driver::{CLIErr, DrvErr};

use self::core::user::Usr;
use self::core::task::TaskRun;
use self::core::kern::{Kern, KernErr};
use self::core::serv::Serv;
use self::core::unit::{Unit, UnitParse};

// use self::serv::{io, sys, math, gfx, dat, time, test};
use self::serv::{test};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    // register service
    let services = [
        // (io::term::SERV_PATH, Box::new(io::term::help_hlr) as Box<ServHlr>, Box::new(io::term::term_hlr) as Box<ServHlr>),
        // (io::store::SERV_PATH, Box::new(io::store::help_hlr) as Box<ServHlr>, Box::new(io::store::store_hlr) as Box<ServHlr>),
        // // ("auto.fsm", Box::new(etc::fsm::FSM::default()) as Box<dyn ServHlr>),
        // (dat::proc::SERV_PATH, Box::new(dat::proc::help_hlr) as Box<ServHlr>, Box::new(dat::proc::proc_hlr) as Box<ServHlr>),
        // (dat::gen::SERV_PATH, Box::new(dat::gen::help_hlr) as Box<ServHlr>, Box::new(dat::gen::gen_hlr) as Box<ServHlr>),
        // (time::chrono::SERV_PATH, Box::new(time::chrono::help_hlr) as Box<ServHlr>, Box::new(time::chrono::chrono_hlr) as Box<ServHlr>),
        // (gfx::gfx2d::SERV_PATH, Box::new(gfx::gfx2d::help_hlr) as Box<ServHlr>, Box::new(gfx::gfx2d::gfx2d_hlr) as Box<ServHlr>),
        // (math::calc::SERV_PATH,  Box::new(math::calc::help_hlr) as Box<ServHlr>, Box::new(math::calc::calc_hlr) as Box<ServHlr>),
        // (sys::task::SERV_PATH, Box::new(sys::task::help_hlr) as Box<ServHlr>, Box::new(sys::task::task_hlr) as Box<ServHlr>),
        // (sys::usr::SERV_PATH, Box::new(sys::usr::help_hlr) as Box<ServHlr>, Box::new(sys::usr::usr_hlr) as Box<ServHlr>),
        // (sys::hw::SERV_PATH, Box::new(sys::hw::help_hlr) as Box<ServHlr>, Box::new(sys::hw::hw_hlr) as Box<ServHlr>),
        // (test::dump::SERV_PATH, Box::new(test::dump::help_hlr) as Box<ServHlr>, Box::new(test::dump::dump_hlr) as Box<ServHlr>),
        (test::echo::SERV_PATH, test::echo::EchoHlr),
        // (test::void::SERV_PATH, Box::new(test::void::help_hlr) as Box<ServHlr>, Box::new(test::void::void_hlr) as Box<ServHlr>)
    ];

    for (name, hlr) in services {
        let serv = Serv::new(name, Box::new(hlr));
        kern.reg_serv(serv)?;

        writeln!(kern, "INFO vnix:kern: service `{}` registered", name).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
    }

    // register user
    let _super = Usr::new("super", &mut kern)?.0;
    kern.reg_usr(_super.clone())?;

    writeln!(kern, "INFO vnix:kern: user `{}` registered", _super).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;

    // test
    // let s = "{
    //     task.stk:[
    //         (set.res.gfx '720p')
    //         cls
    //         (say `loading video..`)
    //         (load @vid.vnix.logo.720p)@io.store
    //         (set.res.gfx '1080p')
    //         cls
    //         (say `loading video..`)
    //         (load @vid.vnix.logo.1080p)@io.store
    //         (say done)
    //     ]@io.term   
    // }";
    // let s = "(task.stk [cls (load @vid.sonic)@io.store (say done)]@io.term)";
    // let s = "{say:{a:[1 {b:c} 3] d:-} nice:4 nl:t}@io.term";
    // let msg = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    let s = "123";
    let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
    let msg = kern.msg("super", u)?;

    let mut mtx = spin::mutex::Mutex::new(kern);

    if let Some(msg) = block_on(Kern::send(&mut mtx, "test.echo".into(), msg))? {
        writeln!(mtx.lock(), "msg: {}", msg).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
    }

    // let run = TaskRun(msg, "test.echo".into());
    // kern.reg_task(&_super.name, "test", run)?;

    // kern.run()

    // // run
    // let path = Unit::parse("@task.init".chars()).map_err(|e| KernErr::ParseErr(e))?.0;
    // let msg = kern.ram_store.load(path).ok_or(KernErr::DbLoadFault)?;

    // let run = TaskRun(msg, "sys.task".into());

    // kern.reg_task(&_super.name, "init.load", run)?;

    // kern.run()
    Ok(())
}
