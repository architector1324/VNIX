pub mod core;
pub mod serv;

use crate::driver::CLIErr;

use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    kern.cli.clear().map_err(|e| KernErr::CLIErr(e))?;

    writeln!(kern.cli, "INFO vnix:kern: ok").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    // register user
    let _super = Usr::new("super")?;
    kern.reg_usr(_super.clone())?;

    writeln!(kern.cli, "INFO vnix:kern: user `{}` registered", _super.name).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    loop {
        // prepare message
        // λ
        let s = "{prs:t inp:`$ ` msg:`Hello, vnix!` task:`io.term`}";
    
        let u0 = Unit::parse(s.chars(), &mut kern)?.0;
        let u = kern.unit(u0)?;
    
        let mut msg = kern.msg("super", u)?;

        // run
        while let Some(_msg) = kern.task(msg)? {
            // writeln!(kern.cli, "INFO vnix:kern: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            msg = _msg;
        }
    }
}
