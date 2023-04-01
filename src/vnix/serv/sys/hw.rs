use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::driver::{DrvErr, MemSizeUnits};

use crate::{thread, thread_await, maybe, as_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitNew, UnitAs, UnitTypeReadAsync};


pub const SERV_PATH: &'static str = "sys.hw";
pub const SERV_HELP: &'static str = "Service for hardware management\nExample: get.mem.free.mb@sys.hw";


fn get_freemem(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<usize> {
    thread!({
        let (s, ath) = maybe!(as_async!(msg, as_str, ath, orig, kern));

        let units = match s.as_str() {
            "get.mem.free" => MemSizeUnits::Bytes,
            "get.mem.free.kb" => MemSizeUnits::Kilo,
            "get.mem.free.mb" => MemSizeUnits::Mega,
            "get.mem.free.gb" => MemSizeUnits::Giga,
            _ => return Ok(None)
        };
        return kern.lock().drv.mem.free(units).map_err(|e| KernErr::DrvErr(DrvErr::Mem(e))).map(|res| Some((res, ath)))
    })
}

pub fn hw_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        if let Some((free_mem, ath)) = thread_await!(get_freemem(Rc::new(msg.ath.clone()), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::int(free_mem as i32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}
