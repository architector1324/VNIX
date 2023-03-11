use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::rc::Rc;
use alloc::string::String;
use spin::Mutex;
use alloc::boxed::Box;

use crate::driver::{Duration, DrvErr};

use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, SchemaPair, SchemaUnit, Schema};
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "time.chrono";
pub const SERV_HELP: &'static str = "Service for time control\nExample: {wait.ms:500}@time.chrono # wait for 0.5 sec.";


fn get_wait(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(Duration, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((res, ath)) = read_async!(msg, ath, orig, kern)? {
            // sec
            if let Some(sec) = res.as_int() {
                return Ok(Some((Duration::Seconds(sec as usize), ath)))
            }

            // (wait.<units> <time>)
            let schm = SchemaPair(SchemaUnit, SchemaUnit);
            if let Some((s, time)) = schm.find(&orig, &res) {
                if let Some((s, ath)) = read_async!(Rc::new(s), ath, orig, kern)?.and_then(|(s, ath)| Some((s.as_str()?, ath))) {
                    if let Some((time, ath)) = read_async!(Rc::new(time), ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_int()?, ath))) {
                        match s.as_str() {
                            "wait" => return Ok(Some((Duration::Seconds(time as usize), ath))),
                            "wait.ms" => return Ok(Some((Duration::Milli(time as usize), ath))),
                            "wait.mcs" => return Ok(Some((Duration::Micro(time as usize), ath))),
                            _ => return Ok(None)
                        }
                    }
                }
            }
        }

        if let Some((sec, ath)) = as_map_find_async!(msg, "wait", ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_int()?, ath))) {
            return Ok(Some((Duration::Seconds(sec as usize), ath)))
        }

        if let Some((ms, ath)) = as_map_find_async!(msg, "wait.ms", ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_int()?, ath))) {
            return Ok(Some((Duration::Milli(ms as usize), ath)))
        }

        if let Some((mcs, ath)) = as_map_find_async!(msg, "wait.mcs", ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_int()?, ath))) {
            return Ok(Some((Duration::Micro(mcs as usize), ath)))
        }

        Ok(None)
    })
}

pub fn chrono_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());
        let ath = Rc::new(msg.ath.clone());

        if let Some((dur, _ath)) = thread_await!(get_wait(ath.clone(), u.clone(), u, kern))? {
            let wait = kern.lock().drv.time.wait_async(dur);
            thread_await!(wait).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;

            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
            }
        }

        Ok(Some(msg))
    })
}
