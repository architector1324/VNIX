use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;

use crate::{thread, thread_await, as_map_find_async, read_async, maybe};

use crate::driver::{CLIErr, DrvErr};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::UnitReadAsyncI;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "test.dump";
pub const SERV_HELP: &'static str = "Test print service\nExample: abc@test.dump";

pub fn dump_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let mut ath = Rc::new(msg.ath.clone());

        let dump = if let Some((dump, _ath)) = as_map_find_async!(msg.msg.clone(), "msg", ath.clone(), msg.msg.clone(), kern)? {
            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
                ath = _ath;
            }
            dump
        } else {
            let (dump, _ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));
            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
                ath = _ath;
            }
            dump
        };

        let task_id = kern.lock().get_task_running();

        for i in 0..5 {
            writeln!(kern.lock().drv.cli, "dump {}[{task_id}] {i}: {}", ath, dump).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Clear)))?;
            yield;
        }
        Ok(Some(msg))
    })    
}
