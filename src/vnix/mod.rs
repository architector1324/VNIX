pub mod core;
pub mod serv;

use self::core::unit::Unit;
use self::core::user::Usr;
use self::core::kern::{Kern, KernErr};


pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    kern.cli.reset().map_err(|e| KernErr::CLIErr(e))?;

    // register user
    let _super = Usr::new("super")?;
    kern.reg_usr(_super)?;

    // prepare message
    let s = "{`fill`:16711680 `msg`:`Hello, vnix ®Ꮘ!`}";

    let u0 = Unit::parse(s.chars(), &mut kern)?.0;
    let u = kern.unit(u0)?;

    let msg = kern.msg("super", u)?;

    // run
    let _ = kern.send("io.term", msg)?;

    loop {

    }
}
