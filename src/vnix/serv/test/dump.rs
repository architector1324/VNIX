use spin::Mutex;
use alloc::boxed::Box;

use crate::{thread, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitModify, UnitParse, UnitAs};


pub const SERV_PATH: &'static str = "test.dump";

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let s = maybe_ok!(msg.msg.clone().as_str());

        let help_s = "{
            name:test.dump
            info:`Dump message to unit service`
            tut:{
                info:`Dump message`
                com:abc@test.dump
                res:{
                    ath:super
                    size:35
                    msg:abc
                    hash:`tTqmP8E+h8YCupEBG9NA9tIQTCUtEBczPpE9jOTthDI=`
                    sign:`M3VaF3AedSnx+/KNXOx2AXIn+8p+nVilbDo68X3dd5d9qMvlXTpSW6FMgw//fPErtg9r7YBcSZFz2i+nCFb0aQ==`
                }
            }
            man:-
        }";
        let help = Unit::parse(help_s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
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
