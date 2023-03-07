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
        if let Some(sec) = thread_await!(msg.clone().read_async(ath.clone(), orig.clone(), kern))?.and_then(|u| u.as_int()) {
            return Ok(Some(Duration::Seconds(sec as usize)))
        }

        if let Some(sec) = thread_await!(msg.clone().as_map_find_async("wait".into(), ath.clone(), orig.clone(), kern))?.and_then(|u| u.as_int()) {
            return Ok(Some(Duration::Seconds(sec as usize)))
        }

        if let Some(ms) = thread_await!(msg.clone().as_map_find_async("wait.ms".into(), ath.clone(), orig.clone(), kern))?.and_then(|u| u.as_int()) {
            return Ok(Some(Duration::Milli(ms as usize)))
        }

        if let Some(mcs) = thread_await!(msg.as_map_find_async("wait.mcs".into(), ath, orig, kern))?.and_then(|u| u.as_int()) {
            return Ok(Some(Duration::Micro(mcs as usize)))
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
