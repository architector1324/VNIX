pub mod core;
pub mod serv;
pub mod utils;
pub mod content;

use alloc::string::String;
use alloc::vec;

use crate::driver::CLIErr;

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

    // prepare ram db
    let content = vec![
        ("task.login", content::task::LOGIN),
        ("task.lambda", content::task::LAMBDA),
        ("img.minecraft.grass", content::img::MINECRAFT_GRASS),
        ("img.vnix.logo", content::img::VNIX_LOGO),
        ("img.wall.ai", content::img::WALL_AI)
    ];

    for (path, s) in content {
        let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
    
        kern.db_ram.save(
            Unit::Str(path.into()),
            u
        );
    }

    // login task
    let mut ath: String = "super".into();

    'login: loop {
        let u = kern.db_ram.load(Unit::Str("task.login".into())).ok_or(KernErr::DbLoadFault)?;
        let msg = kern.msg("super", u)?;
    
        let go = kern.task(msg);

        match go {
            Err(e) => writeln!(kern.cli, "ERR vnix:kern: failed to login {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?,
            Ok(msg) => {
                if let Some(msg) = msg {
                    ath = msg.ath;
                    break 'login;
                }
            }
        }
    }

    loop {
        // prepare message
        // λ
        let u = kern.db_ram.load(Unit::Str("task.lambda".into())).ok_or(KernErr::DbLoadFault)?;
        let msg = kern.msg(&ath, u)?;

        // run
        if let Err(e) = kern.task(msg) {
            writeln!(kern.cli, "ERR vnix:kern: {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        }
    }
}
