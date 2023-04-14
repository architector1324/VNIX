use spin::Mutex;
use alloc::boxed::Box;

use crate::{thread, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitModify, UnitAs};


pub const SERV_PATH: &'static str = "test.dump";

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
                Unit::str("Dump message to unit service")
            ),
            (
                Unit::str("tut"),
                Unit::map(&[
                    (Unit::str("info"), Unit::str("Dump message")),
                    (Unit::str("com"), Unit::stream_loc(Unit::str("abc"), "test.dump")),
                    (
                        Unit::str("res"),
                        Unit::map(&[
                            (Unit::str("ath"), Unit::str("super")),
                            (Unit::str("size"), Unit::uint(35)),
                            (Unit::str("msg"), Unit::str("abc")),
                            (Unit::str("hash"), Unit::str("tTqmP8E+h8YCupEBG9NA9tIQTCUtEBczPpE9jOTthDI=")),
                            (Unit::str("sign"), Unit::str("M3VaF3AedSnx+/KNXOx2AXIn+8p+nVilbDo68X3dd5d9qMvlXTpSW6FMgw//fPErtg9r7YBcSZFz2i+nCFb0aQ==")),
                        ])
                    )
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
