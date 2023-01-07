use heapless::{LinearMap, Vec};

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
    let u0 = Unit::Str("msg".into());
    let u1 = Unit::Str("Hello, vnix ®!".into());

    let u2 = Unit::Str("a".into());
    let u3 = Unit::None;

    let u4 = Unit::Int(1);
    let u5 = Unit::Dec(3.14);

    let mut u6_lst = Vec::new();
    u6_lst.push(&u2).expect("memory out!");
    u6_lst.push(&u4).expect("memory out!");
    u6_lst.push(&u5).expect("memory out!");

    let u6 = Unit::Lst(u6_lst);
    let u7 = Unit::Str("b".into());

    let mut u8_map = LinearMap::new();
    u8_map.insert(&u0, &u1).expect("cannot construct msg!");
    u8_map.insert(&u2, &u3).expect("cannot construct msg!");
    u8_map.insert(&u7, &u6).expect("cannot construct msg!");

    let msg = Msg {
        ath: _super,
        msg: Unit::Map(u8_map)
    };

    // run
    cli.println(core::format_args!("{}", msg));

    loop {

    }
}
