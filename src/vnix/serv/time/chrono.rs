use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use spin::Mutex;
use async_trait::async_trait;

use crate::vnix::core::driver::{DrvErr, Duration, Time, TimeUnit};

use crate::vnix::core::task::Yield;
use crate::{as_async, as_map_find_as_async, maybe, maybe_ok, read_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitTypeAsyncResult, UnitNew};


pub const SERV_PATH: &'static str = "time.chrono";

pub const SERV_HELP: &'static str = "{
    name:time.chrono
    info:`Service for time managment`
    tut:[
        {
            info:`Pause task for specified duration`
            com:[
                (wait 1)@time.chrono
                (wait.ms 500)@time.chrono
                (wait.mcs 2000000)@time.chrono
            ]
        }
        {
            info:`Get system uptime in minutes`
            com:get.up.min@time.chrono
            res:5
        }
        {
            info:`Measure unit read time in seconds`
            com:[
                (bch {fac:123456}@math.calc)@time.chrono
                (bch.sec {fac:123456}@math.calc)@time.chrono
            ]
            res:4
        }
    ]
    man:{
        wait:{
            info:`Pause task for specified duration`
            units:[mcs ms sec min hour day week mnh year]
            schm:[
                uint
                (wait uint)
                (`wait.<units>` uint)
                {wait:uint}
                {`wait.<units>`:uint}
            ]
            tut:@tut.0
        }
        get.up:{
            info:`Get system uptime`
            units:[mcs ms sec min hour day week mnh year]
            schm:[
                get.up
                `get.up.<units>`
            ]
            tut:@tut.1
        }
        bch:{
            info:`Measure unit read time`
            units:[mcs ms sec min hour day week mnh year]
            schm:[
                (bch unit)
                (`bch.<units>` unit)
            ]
            tut:@tut.2
        }
    }
}";

pub struct ChronoHlr;

impl ChronoHlr {
    async fn wait(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeAsyncResult<Duration> {
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
    }

    async fn bench(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeAsyncResult<usize> {
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
    }

    async fn get_up(ath: Rc<String>, _orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeAsyncResult<usize> {
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
        Yield::now().await;

        Ok(Some((up as usize, ath)))
    }
}

#[async_trait(?Send)]
impl ServHlr for ChronoHlr {
    async fn hlr(&self, mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // wait
        if let Some((dur, _ath)) = Self::wait(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let time = unsafe {
                let lck = kern.lock();
                let time = &lck.drv.time as *const Box<dyn Time>;
                &*time
            };

            time.wait_async(dur).await.map_err(|e| KernErr::DrvErr(DrvErr::Time(e)))?;

            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
                return Ok(Some(msg))
            }
        }

        // up
        if let Some((elapsed, _ath)) = Self::get_up(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(elapsed as u32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // bench
        if let Some((elapsed, _ath)) = Self::bench(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(elapsed as u32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    }
}
