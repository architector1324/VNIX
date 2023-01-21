pub mod core;
pub mod serv;

use alloc::vec;
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

    // login task
    let mut ath: String = "super".into();

    'login: loop {
        let s = "{prs:t inp:`login:` msg:`Hello, vnix!` prs:t ath:@msg.ath pub:@msg.pub priv:@msg.priv task:[io.term sys.usr]}";

        let u = Unit::parse(s.chars(), &mut kern)?.0;
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
        let s = "{prs:t inp:`$ ` msg:`Welcome to lambda shell!` task:[io.term sys.task io.term]}";

        let u = Unit::parse(s.chars(), &mut kern)?.0;
        let msg = kern.msg(&ath, u)?;

        // run
        if let Err(e) = kern.task(msg) {
            writeln!(kern.cli, "ERR vnix:kern: {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        }
    }
}
