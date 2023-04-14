use spin::Mutex;
use alloc::boxed::Box;

use crate::thread;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::unit::{Unit, UnitNew};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.dump";

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let help = Unit::map(&[
            (
                Unit::str("name"),
                Unit::str(SERV_PATH)
            ),
            (
                Unit::str("info"),
                Unit::str("Dump message to unit service\nExample: abc@test.dump")
            )
        ]);
        yield;

        let _msg = Unit::map(&[
            (Unit::str("msg"), help)
        ]);
        kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
    })
}

pub fn dump_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Unit::map(&[
            (Unit::str("ath"), Unit::str(&msg.ath)),
            (Unit::str("size"), Unit::uint(msg.size as u32)),
            (Unit::str("msg"), msg.msg.clone()),
            (Unit::str("hash"), Unit::str(&msg.hash)),
            (Unit::str("sign"), Unit::str(&msg.sign)),
        ]);

        let _msg = Unit::map(&[
            (Unit::str("msg"), u)
        ]);
        yield;

        return kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
    })    
}
