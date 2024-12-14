use spin::Mutex;

use async_trait::async_trait;

use crate::vnix::utils::Maybe;
use crate::maybe_ok;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlr, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitModify, UnitParse, UnitAs};


pub const SERV_PATH: &'static str = "test.echo";

pub struct EchoHlr;

#[async_trait(?Send)]
impl ServHlr for EchoHlr {
    async fn help_hlr(&self, msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> Maybe<Msg, KernErr> {
        let s = maybe_ok!(msg.msg.clone().as_str());
    
        let help_s = "{
            name:test.echo
            info:`Test echo service`
            tut:{
                info:`Echo message`
                com:a@test.echo
                res:a
            }
            man:-
        }";
        let help = Unit::parse(help_s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
        async {}.await;
    
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
    }
    
    async fn hlr(&self, msg: Msg, _serv: ServInfo, _kern: &Mutex<Kern>) -> Maybe<Msg, KernErr> {
        Ok(Some(msg))
    }
}
