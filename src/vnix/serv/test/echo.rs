use spin::Mutex;

use async_trait::async_trait;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};


pub const SERV_PATH: &'static str = "test.echo";

pub const SERV_HELP: &'static str = "{
    name:test.echo
    info:`Test echo service`
    tut:{
        info:`Echo message`
        com:a@test.echo
        res:a
    }
    man:-
}";

pub struct EchoHlr;

#[async_trait(?Send)]
impl ServHlr for EchoHlr {
    async fn hlr(&self, msg: Msg, _serv: ServInfo, _kern: &Mutex<Kern>) -> ServResult {
        Ok(Some(msg))
    }
}
