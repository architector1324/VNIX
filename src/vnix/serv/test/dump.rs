use spin::Mutex;

use async_trait::async_trait;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};
use crate::vnix::core::unit::{Unit, UnitNew};


pub const SERV_PATH: &'static str = "test.dump";

pub const SERV_HELP: &'static str = "{
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

pub struct DumpHlr;


#[async_trait(?Send)]
impl ServHlr for DumpHlr {
    async fn hlr(&self, msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
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
        async{}.await;

        return kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
    }
}
