pub mod core;
pub mod serv;
pub mod utils;

use alloc::string::String;
use alloc::vec;

use crate::driver::CLIErr;
use crate::vnix::core::unit::DisplayShort;

use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};
use self::core::serv::{Serv, ServKind};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    // register service
    let services = vec![
        ("io.term", ServKind::IOTerm),
        ("io.store", ServKind::IOStore),
        ("etc.chrono", ServKind::EtcChrono),
        ("etc.fsm", ServKind::EtcFSM),
        ("gfx.2d", ServKind::GFX2D),
        ("math.int", ServKind::MathInt),
        ("sys.task", ServKind::SysTask),
        ("sys.usr", ServKind::SysUsr),
    ];

    for (name, kind) in services {
        let serv = Serv::new(name, kind);
        kern.reg_serv(serv)?;

        writeln!(kern.cli, "INFO vnix:kern: service `{}` registered", name).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    }

    // register user
    let _super = Usr::new("super", &mut kern)?.0;
    kern.reg_usr(_super.clone())?;

    writeln!(kern.cli, "INFO vnix:kern: user `{}` registered", _super).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    // test
    let s = "{
        term.gfx:[
            (set.res.gfx (1920 1080))
            (load @img.minecraft.grass)@io.store
            key
            (load @img.vnix.logo)@io.store
            key
            (load @img.wall.ai.0)@io.store
            key
            (load @img.wall.ai.1)@io.store
            key
            (load @img.wall.ai.2)@io.store
            key
            (load @img.wall.ai.3)@io.store
            key
            (load @img.wall.ai.4)@io.store
            key
            (load @img.wall.ai.5)@io.store
            key
            (load @img.wall.ai.6)@io.store
            key
            (load @img.wall.elk)@io.store
            key
            (load @img.wall.triangles)@io.store
            key
            (load @img.wall.cubes)@io.store
            key
            (load @img.wall.lines)@io.store
            key
            (load @img.wall.spirals)@io.store
            key
            (load @img.wall.green_blue)@io.store
            key
        ]
    }";
    let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    let msg = kern.msg("super", u)?;
    
    if let Some(msg) = kern.send("io.term", msg)? {
        writeln!(kern.cli, "{}", DisplayShort(&msg.msg, 16)).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    }

    Ok(())

    // // login task
    // let mut ath: String = "super".into();

    // 'login: loop {
    //     let path = Unit::parse("@task.zen.login".chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    //     let u = kern.ram_store.load(path).ok_or(KernErr::DbLoadFault)?;
    //     let msg = kern.msg("super", u)?;
    
    //     let go = kern.task(msg);

    //     match go {
    //         Err(e) => writeln!(kern.cli, "ERR vnix:kern: failed to login {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?,
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
    //         writeln!(kern.cli, "ERR vnix:kern: {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    //     }
    // }
}
