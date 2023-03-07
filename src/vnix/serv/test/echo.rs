use spin::Mutex;
use alloc::boxed::Box;

use crate::thread;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.echo";
pub const SERV_HELP: &'static str = "Test echo service\nExample: abc@test.echo";

pub fn echo_hlr(msg: Msg, _serv: ServInfo, _kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        yield;
        Ok(Some(msg))
    })    
}
