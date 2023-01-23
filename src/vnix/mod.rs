pub mod core;
pub mod serv;
pub mod utils;
pub mod content;

use alloc::string::String;

use crate::driver::CLIErr;

use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    writeln!(kern.cli, "INFO vnix:kern: ok").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    // register user
    let _super = Usr::new("super", &mut kern)?.0;
    kern.reg_usr(_super.clone())?;

    writeln!(kern.cli, "INFO vnix:kern: user `{}` registered", _super).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    // prepare ram db
    let s = content::task::LOGIN;
    let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    kern.db_ram.save(
        Unit::Str("task.login".into()),
        u
    );

    let s = content::task::LAMBDA;
    let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    kern.db_ram.save(
        Unit::Str("task.lambda".into()),
        u
    );

    let s = content::img::MINECRAFT_GRASS;
    let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    kern.db_ram.save(
        Unit::Str("img.minecraft.grass".into()),
        u
    );

    let s = content::img::VNIX_LOGO;
    let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    kern.db_ram.save(
        Unit::Str("img.vnix.logo".into()),
        u
    );

    let s = content::img::WALL_AI;

    let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;

    kern.db_ram.save(
        Unit::Str("img.wall.ai".into()),
        u
    );

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
