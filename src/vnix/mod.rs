pub mod core;
pub mod serv;

use crate::driver::CLIErr;

use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    kern.cli.clear().map_err(|e| KernErr::CLIErr(e))?;

    // register user
    let _super = Usr::new("super")?;
    kern.reg_usr(_super)?;

    // prepare message
    // λ
    let s = "{prs:t inp:`$ ` msg:`Hello, vnix ®Ꮘ!`}";

    let u0 = Unit::parse(s.chars(), &mut kern)?.0;
    let u = kern.unit(u0)?;

    let msg = kern.msg("super", u)?;

    // run
    if let Some(msg) = kern.send("io.term", msg)? {
        writeln!(kern.cli, "INFO vnix:kern: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    }

    loop {

    }
}
