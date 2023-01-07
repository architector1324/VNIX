pub mod unit;
pub mod msg;
pub mod user;
pub mod serv;
pub mod kern;

use unit::Unit;
use msg::Msg;
use user::Usr;
use kern::Kern;

use self::kern::KernErr;

pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    kern.cli.reset().map_err(|e| KernErr::CLIErr(e))?;

    // prepare user
    let _super = Usr {
        name: "super".into()
    };

    // prepare message
    let s = "`Hello, vnix ®!`";
    let u0 = Unit::parse(s, &mut kern)?;
    let u = kern.unit(u0)?;

    let msg = Msg {
        ath: _super,
        msg: u
    };

    // run
    kern.cli.println(core::format_args!("{}", msg));
    kern.cli.println(core::format_args!("{:?}", msg));

    loop {

    }
}
