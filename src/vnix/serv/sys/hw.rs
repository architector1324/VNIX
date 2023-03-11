use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::vec;
use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::{DrvErr, MemSizeUnits};

use crate::vnix::core::task::ThreadAsync;
use crate::{thread, thread_await, read_async};
use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "sys.hw";
pub const SERV_HELP: &'static str = "Service for hardware management\nExample: get.mem.free.mb@sys.hw";


fn get_freemem(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(usize, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((s, ath)) = read_async!(msg, ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_str()?, ath))) {
            let units = match s.as_str() {
                "get.mem.free" => MemSizeUnits::Bytes,
                "get.mem.free.kb" => MemSizeUnits::Kilo,
                "get.mem.free.mb" => MemSizeUnits::Mega,
                "get.mem.free.gb" => MemSizeUnits::Giga,
                _ => return Ok(None)
            };

            return kern.lock().drv.mem.free(units).map_err(|e| KernErr::DrvErr(DrvErr::Mem(e))).map(|res| Some((res, ath)))
        }
        Ok(None)
    })
}

pub fn hw_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());

        if let Some((free_mem, ath)) = thread_await!(get_freemem(Rc::new(msg.ath.clone()), u.clone(), u, kern))? {
            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), Unit::Int(free_mem as i32))]
            );

            let _msg = msg.msg.merge(m);
            return kern.lock().msg(&ath, _msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}
