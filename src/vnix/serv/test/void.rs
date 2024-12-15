use spin::Mutex;

use async_trait::async_trait;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};


pub const SERV_PATH: &'static str = "test.void";

pub const SERV_HELP: &'static str = "{
    name:test.void
    info:`'Black hole' service`
    tut:{
        info:`Destroy message`
        com:a@test.void
    }
    man:-
}";

pub struct VoidHlr;

#[async_trait(?Send)]
impl ServHlr for VoidHlr {
    async fn hlr(&self, _msg: Msg, _serv: ServInfo, _kern: &Mutex<Kern>) -> ServResult {
        Ok(None)
    }
}
