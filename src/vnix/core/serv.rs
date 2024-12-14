use alloc::boxed::Box;
use alloc::string::String;

use async_trait::async_trait;

use crate::maybe_ok;
use crate::vnix::utils::Maybe;

use super::unit::{Unit, UnitNew, UnitModify, UnitParse, UnitAs};

use super::msg::Msg;
use super::kern::{KernErr, Kern};

use spin::Mutex;


pub type ServResult = Maybe<Msg, KernErr>;

#[async_trait(?Send)]
pub trait ServHlr {
    async fn help_hlr(&self, msg: Msg, serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
        let s = maybe_ok!(msg.msg.clone().as_str());

        let help = Unit::parse(serv.help.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
        async{}.await;

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

    async fn hlr(&self, msg: Msg, serv: ServInfo, kern: &Mutex<Kern>) -> ServResult;
}

#[derive(Debug)]
pub enum ServErr {
    NotValidUnit
}

#[derive(Debug, Clone)]
pub struct ServInfo {
    pub name: String,
    pub help: String,
}

pub struct Serv {
    pub info: ServInfo,
    pub hlr: Box<dyn ServHlr>
}


impl Serv {
    pub fn new(name: &str, help: &str, hlr: Box<dyn ServHlr>) -> Self {
        Serv {
            info: ServInfo {
                name: name.into(),
                help: help.into()
            },
            hlr
        }
    }
}
