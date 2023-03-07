use spin::Mutex;
use alloc::boxed::Box;

use crate::driver::{CLIErr, DrvErr};
use crate::thread;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.dumb";
pub const SERV_HELP: &'static str = "Test echo service\nExample: abc@test.dumb";

pub fn dumb_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let task_id = kern.lock().get_task_running();

        for i in 0..5 {
            writeln!(kern.lock().drv.cli, "test[{task_id}] {i}: {}", msg.msg).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Clear)))?;
            yield;
        }
        Ok(Some(msg))
    })    
}
