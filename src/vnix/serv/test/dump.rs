use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;

use crate::{thread, thread_await, as_map_find_async};

use crate::driver::{CLIErr, DrvErr};
use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.dump";
pub const SERV_HELP: &'static str = "Test print service\nExample: abc@test.dump";

pub fn dump_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());
        let ath = Rc::new(msg.ath.clone());

        let dump = if let Some(dump) = as_map_find_async!(u.clone(), "msg", ath.clone(), u.clone(), kern)? {
            dump
        } else {
            Rc::unwrap_or_clone(u)
        };

        let task_id = kern.lock().get_task_running();

        for i in 0..5 {
            writeln!(kern.lock().drv.cli, "dump {}[{task_id}] {i}: {}", ath, dump).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Clear)))?;
            yield;
        }
        Ok(Some(msg))
    })    
}
