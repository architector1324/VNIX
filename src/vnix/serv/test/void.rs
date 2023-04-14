use spin::Mutex;
use alloc::boxed::Box;

use crate::{thread, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::unit::{Unit, UnitNew, UnitModify, UnitAs};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.void";

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let s = maybe_ok!(msg.msg.clone().as_str());

        let help = Unit::map(&[
            (
                Unit::str("name"),
                Unit::str(SERV_PATH)
            ),
            (
                Unit::str("info"),
                Unit::str("'Black hole' service")
            ),
            (
                Unit::str("tut"),
                Unit::map(&[
                    (Unit::str("info"), Unit::str("Destroy message")),
                    (Unit::str("com"), Unit::stream_loc(Unit::str("a"), "test.void"))
                ])
            ),
            (Unit::str("man"), Unit::none())
        ]);
        yield;

        let res = match s.as_str() {
            "help" => help,
            "help.name" => maybe_ok!(help.find(["name"].into_iter())),
            "help.info" => maybe_ok!(help.find(["info"].into_iter())),
            "help.tut" => maybe_ok!(help.find(["tut"].into_iter())),
            "help.man" => maybe_ok!(help.find(["man"].into_iter())),
            _ => return Ok(None)
        };

        let _msg = Unit::map(&[
            (Unit::str("msg"), res)
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
