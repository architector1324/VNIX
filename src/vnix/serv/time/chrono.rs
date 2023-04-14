use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::rc::Rc;
use alloc::string::String;
use spin::Mutex;
use alloc::boxed::Box;

use crate::vnix::core::driver::{Duration, DrvErr, TimeUnit};

use crate::{thread, thread_await, as_map_find_as_async, as_async, read_async, maybe, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitTypeReadAsync, UnitNew};


pub const SERV_PATH: &'static str = "time.chrono";

fn wait(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Duration> {
    thread!({
        // sec
        if let Some(sec) = msg.clone().as_uint() {
            return Ok(Some((Duration::Seconds(sec as usize), ath)))
        }

        // (wait.<units> <time>)
        if let Some((s, time)) = msg.clone().as_pair() {
            let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));
            let (time, ath) = maybe!(as_async!(time, as_uint, ath, orig, kern));

            match s.as_str() {
                "wait" | "wait.sec" => return Ok(Some((Duration::Seconds(time as usize), ath))),
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

fn bench(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<usize> {
    thread!({
        // (bch.<units> <unit>)
        let (s, u) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        let units = match s.as_str() {
            "bch.mcs" => TimeUnit::Micro,
            "bch.ms" => TimeUnit::Milli,
            "bch" | "bch.sec" => TimeUnit::Second,
            "bch.min" => TimeUnit::Minute,
            "bch.hour" => TimeUnit::Hour,
            "bch.day" => TimeUnit::Day,
            "bch.week" => TimeUnit::Week,
            "bch.mnh" => TimeUnit::Month,
            "bch.year" => TimeUnit::Year,
            _ => return Ok(None)
        };

        let start = kern.lock().drv.time.uptime(units).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;
        maybe!(read_async!(u, ath, orig, kern));

        let end = kern.lock().drv.time.uptime(units).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;
        let elapsed = (end - start) as usize;

        Ok(Some((elapsed, ath)))
    })
}

fn get_up(ath: Rc<String>, _orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<usize> {
    thread!({
        // up.<units>
        let s = maybe_ok!(msg.as_str());

        let units = match s.as_str() {
            "get.up.mcs" => TimeUnit::Micro,
            "get.up.ms" => TimeUnit::Milli,
            "get.up" | "get.up.sec" => TimeUnit::Second,
            "get.up.min" => TimeUnit::Minute,
            "get.up.hour" => TimeUnit::Hour,
            "get.up.day" => TimeUnit::Day,
            "get.up.week" => TimeUnit::Week,
            "get.up.mnh" => TimeUnit::Month,
            "get.up.year" => TimeUnit::Year,
            _ => return Ok(None)
        };

        let up = kern.lock().drv.time.uptime(units).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;
        yield;

        Ok(Some((up as usize, ath)))
    })
}

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let help = Unit::map(&[
            (
                Unit::str("name"),
                Unit::str(SERV_PATH)
            ),
            (
                Unit::str("info"),
                Unit::str("Service for time control\nExample: {wait.ms:500}@time.chrono # wait for 0.5 sec.")
            )
        ]);
        yield;

        let _msg = Unit::map(&[
            (Unit::str("msg"), help)
        ]);
        kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
    })
}

pub fn chrono_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // wait
        if let Some((dur, _ath)) = thread_await!(wait(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let wait = kern.lock().drv.time.wait_async(dur);
            thread_await!(wait).map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;

            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
                return Ok(Some(msg))
            }
        }

        // up
        if let Some((elapsed, _ath)) = thread_await!(get_up(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(elapsed as u32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // bench
        if let Some((elapsed, _ath)) = thread_await!(bench(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(elapsed as u32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}
