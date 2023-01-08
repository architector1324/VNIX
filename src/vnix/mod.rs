pub mod unit;
pub mod msg;
pub mod user;
pub mod serv;
pub mod kern;

use unit::Unit;
use msg::Msg;
use user::Usr;
use kern::{Kern, KernErr};

use crate::driver::CLIErr;

pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    kern.cli.reset().map_err(|e| KernErr::CLIErr(e))?;

    // prepare user
    let _super = Usr {
        name: "super".into()
    };

    // prepare message
    let s = "{`msg`:`Hello, vnix ®!` `a`:- `b`:[`a` 1 3.14 (`c` 2.74)]}";
    let u0 = Unit::parse(s.chars(), &mut kern)?.0;
    let u = kern.unit(u0)?;

    let msg = Msg {
        ath: _super,
        msg: u
    };

    // run
    writeln!(kern.cli, "INFO vnix:kern: {}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
    writeln!(kern.cli, "DEBG vnix:kern: {:?}", msg).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

    loop {

    }
}
