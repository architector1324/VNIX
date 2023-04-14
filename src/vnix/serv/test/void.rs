use spin::Mutex;
use alloc::boxed::Box;

use crate::thread;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::unit::{Unit, UnitNew};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.void";

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let help = Unit::map(&[
            (
                Unit::str("name"),
                Unit::str(SERV_PATH)
            ),
            (
                Unit::str("info"),
                Unit::str("`Black hole` service\nExample: a@test.void")
            )
        ]);
        yield;

        let _msg = Unit::map(&[
            (Unit::str("msg"), help)
        ]);
        kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
    })
}

pub fn void_hlr(_msg: Msg, _serv: ServInfo, _kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        yield;
        Ok(None)
    })    
}
