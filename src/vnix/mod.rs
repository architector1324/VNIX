pub mod core;
pub mod serv;
pub mod utils;

use ::core::ops::{Generator, GeneratorState};
use ::core::pin::Pin;

use alloc::string::String;
use alloc::vec::Vec;

use crate::driver::CLIErr;
use crate::vnix::core::unit::DisplayShort;

use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};
use self::core::serv::{Serv, ServKind};

use spin::Mutex;


pub fn vnix_entry(kern: Kern) -> Result<(), KernErr> {
    let kern_mtx = Mutex::new(kern);

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
        kern_mtx.lock().reg_serv(serv)?;

        writeln!(kern_mtx.lock().drv.cli, "INFO vnix:kern: service `{}` registered", name).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    }

    // register user
    let _super = Usr::new("super", &mut kern_mtx.lock())?.0;
    kern_mtx.lock().reg_usr(_super.clone())?;

    writeln!(kern_mtx.lock().drv.cli, "INFO vnix:kern: user `{}` registered", _super).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    // test
    let s = ["a", "b"];
    let msg = s.map(|s| {
        let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e)).unwrap().0;
        kern_mtx.lock().msg("super", u).unwrap()
    });

    let mut queue = msg.map(|msg| {
        Kern::send(&kern_mtx, "test.dumb.loop", msg)
    });

    loop {
        for q in &mut queue {
            if let GeneratorState::Complete(res) = Pin::new(q).resume(()) {
                res?;
            }
        }
    }

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
