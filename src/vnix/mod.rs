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
        // ("io.term", ServKind::IOTerm),
        // ("io.store", ServKind::IOStore),
        // ("etc.chrono", ServKind::EtcChrono),
        // ("etc.fsm", ServKind::EtcFSM),
        // ("gfx.2d", ServKind::GFX2D),
        // ("math.int", ServKind::MathInt),
        // ("sys.task", ServKind::SysTask),
        // ("sys.usr", ServKind::SysUsr),
        ("test.dumb", ServKind::TestDumb),
        ("test.dumb.loop", ServKind::TestDumbLoop)
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
    let task = TaskLoop::Chain {
        msg: Unit::Str("a".into()),
        chain: vec!["test.dumb".into(), "test.dumb.loop".into(), "test.dumb".into()]
    };

    kern.reg_task(&_super.name, "test", task)?;
    kern.run()

    // // login task
    // let mut ath: String = "super".into();

    // 'login: loop {
    //     let path = Unit::parse("@task.zen.login".chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    //     let u = kern.ram_store.load(path).ok_or(KernErr::DbLoadFault)?;
    //     let msg = kern.msg("super", u)?;
    
    //     let go = kern.task(msg);

    //     match go {
    //         Err(e) => writeln!(kern.drv.cli, "ERR vnix:kern: failed to login {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?,
    //         Ok(msg) => {
    //             if let Some(msg) = msg {
    //                 ath = msg.ath;
    //                 break 'login;
    //             }
    //         }
    //     }
    // }

    // // zen
    // let path = Unit::parse("@task.zen.desk.load".chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    // let u = kern.ram_store.load(path).ok_or(KernErr::DbLoadFault)?;
    // let msg = kern.msg(&ath, u)?;

    // kern.task(msg)?;

    // // λ
    // loop {
    //     let path = Unit::parse("@task.lambda.gfx.load".chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    //     let u = kern.ram_store.load(path).ok_or(KernErr::DbLoadFault)?;
    //     let msg = kern.msg(&ath, u)?;

    //     // run
    //     if let Err(e) = kern.task(msg) {
    //         writeln!(kern.drv.cli, "ERR vnix:kern: {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    //     }
    // }
}
