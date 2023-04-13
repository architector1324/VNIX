use spin::Mutex;
use alloc::boxed::Box;

use crate::thread;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.void";
pub const SERV_HELP: &'static str = "`Black hole` service\nExample: a@test.void";

pub fn void_hlr(_msg: Msg, _serv: ServInfo, _kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        yield;
        Ok(None)
    })    
}
