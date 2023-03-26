use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::rc::Rc;
use alloc::string::String;
use spin::Mutex;
use alloc::boxed::Box;

use crate::driver::{Duration, DrvErr};

use crate::{thread, thread_await, as_map_find_as_async, as_async, maybe};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitTypeReadAsync};


pub const SERV_PATH: &'static str = "time.chrono";
pub const SERV_HELP: &'static str = "Service for time control\nExample: {wait.ms:500}@time.chrono # wait for 0.5 sec.";


fn get_wait(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Duration> {
    thread!({
        // sec
        if let Some((sec, ath)) = as_async!(msg, as_uint, ath, orig, kern)? {
            return Ok(Some((Duration::Seconds(sec as usize), ath)))
        }

        // (wait.<units> <time>)
        if let Some(((s, time), ath)) = as_async!(msg, as_pair, ath, orig, kern)? {
            let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));
            let (time, ath) = maybe!(as_async!(time, as_uint, ath, orig, kern));

            match s.as_str() {
                "wait" => return Ok(Some((Duration::Seconds(time as usize), ath))),
                "wait.ms" => return Ok(Some((Duration::Milli(time as usize), ath))),
                "wait.mcs" => return Ok(Some((Duration::Micro(time as usize), ath))),
                _ => return Ok(None)
            }
        }

        if let Some((sec, ath)) = as_map_find_as_async!(msg, "wait", as_uint, ath, orig, kern)? {
            return Ok(Some((Duration::Seconds(sec as usize), ath)))
        }

        if let Some((ms, ath)) = as_map_find_as_async!(msg, "wait.ms", as_uint, ath, orig, kern)? {
            return Ok(Some((Duration::Milli(ms as usize), ath)))
        }

        if let Some((mcs, ath)) = as_map_find_as_async!(msg, "wait.mcs", as_uint, ath, orig, kern)? {
            return Ok(Some((Duration::Micro(mcs as usize), ath)))
        }

        Ok(None)
    })
}

pub fn chrono_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());

        if let Some((dur, _ath)) = thread_await!(get_wait(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let wait = kern.lock().drv.time.wait_async(dur);
            thread_await!(wait).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;

            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
            }
        }

        Ok(Some(msg))
    })
}
