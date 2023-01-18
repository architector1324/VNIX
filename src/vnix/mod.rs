pub mod core;
pub mod serv;

use crate::driver::CLIErr;

use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    writeln!(kern.cli, "INFO vnix:kern: ok").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    // register user
    let _super = Usr::new("super", &mut kern)?;
    kern.reg_usr(_super.clone())?;

    writeln!(kern.cli, "INFO vnix:kern: user `{}` registered", _super).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    let arch_u = Usr::guest("arch1324", "AoaHgZNb2bv8ftQLvVtdghXATGUJjSOdSfIy31DB9EHU")?;
    kern.reg_usr(arch_u.clone())?;

    writeln!(kern.cli, "INFO vnix:kern: user `{}` registered", arch_u).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    loop {
        // prepare message
        // λ
        let s = "{prs:t inp:`$ ` msg:`Hello, vnix!` task:io.term}";

        let u = Unit::parse(s.chars(), &mut kern)?.0;
        let msg = kern.msg("super", u)?;

        // run
        let go = kern.task(msg);

        if let Err(e) = go {
            writeln!(kern.cli, "ERR vnix:kern: {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
        } else if let Ok(msg) = go {
            if let Some(msg) = msg {
                // writeln!(kern.cli, "DEBG vnix:kern: {:?}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    
                if let Err(e) = kern.task(msg) {
                    writeln!(kern.cli, "ERR vnix:kern: {:?}", e).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                }
            }
        }
    }
}
