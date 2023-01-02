use heapless::LinearMap;

use super::driver::CLI;

pub mod unit;
pub mod msg;
pub mod user;
pub mod serv;
pub mod kern;

use unit::Unit;
use msg::Msg;
use user::Usr;

pub fn vnix_entry(cli: &mut dyn CLI) {
    cli.reset().expect("cannot reset cli!");

    // prepare user
    let _super = Usr {
        name: "super".into()
    };

    // prepare message
    let mut msg = Msg {
        ath: _super,
        msg: LinearMap::new()
    };

    msg.msg.insert(Unit::Str("msg".into()), Unit::Str("Hello, vnix ®!".into())).expect("cannot construct msg!");
    msg.msg.insert(Unit::Str("a".into()), Unit::None).expect("cannot construct msg!");

    // run
    cli.println(core::format_args!("{}", msg));

    loop {

    }
}
