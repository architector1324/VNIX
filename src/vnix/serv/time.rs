use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::rc::Rc;
use alloc::string::String;
use spin::Mutex;
use alloc::boxed::Box;

use crate::driver::{Duration, DrvErr};

use crate::vnix::core::task::ThreadAsync;
use crate::{thread, thread_await};
use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "time.chrono";
pub const SERV_HELP: &'static str = "Service for time control\nExample: {wait.ms:500}@time.chrono # wait for 0.5 sec.";


fn get_wait(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<Duration>, KernErr>> {
    thread!({
        if let Some(msg) = thread_await!(msg.read_async(ath.clone(), orig.clone(), kern))? {
            if let Some(sec) = msg.as_int() {
                return Ok(Some(Duration::Seconds(sec as usize)))
            }

            if let Some(msg) = msg.as_map_find("wait") {
                return thread_await!(get_wait(ath, orig.clone(), Rc::new(msg), kern))
            }

            // if let Some(ms) = msg.as_map_find("wait.ms") {
            //     return Some(Duration::Milli(ms as usize))
            // }
        
            // if let Some(mcs) = msg.as_map_find("wait.mcs") {
            //     return Some(Duration::Micro(mcs as usize))
            // }
        }

        Ok(None)
    })
}

pub fn chrono_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());

        if let Some(dur) = thread_await!(get_wait(Rc::new(msg.ath.clone()), u.clone(), u, kern))? {
            let wait = kern.lock().drv.time.wait_async(dur);
            thread_await!(wait).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;
        }

        Ok(Some(msg))
    })
}
