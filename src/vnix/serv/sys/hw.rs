use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use spin::Mutex;
use async_trait::async_trait;

use crate::{maybe, as_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::driver::{DrvErr, MemSizeUnits};
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitNew, UnitAs, UnitTypeAsyncResult};


pub const SERV_PATH: &'static str = "sys.hw";

pub const SERV_HELP: &'static str = "{
    name:sys.hw
    info:`Service for hardware management`
    tut:[
        {
            info:`Get free RAM space`
            com:get.mem.free.mb@sys.hw
            res:512
        }
    ]
    man:{
        get.mem.free:{
            info:`Get free RAM space`
            size:[kb mb gb]
            schm:[
                get.mem.free
                `get.mem.free.<size>`
            ]
            tut:@tut.0
        }
    }
}";

pub struct HWHlr;

impl HWHlr {
    async fn get_freemem(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeAsyncResult<usize> {
        let (s, ath) = maybe!(as_async!(msg, as_str, ath, orig, kern));

        let units = match s.as_str() {
            "get.mem.free" => MemSizeUnits::Bytes,
            "get.mem.free.kb" => MemSizeUnits::Kilo,
            "get.mem.free.mb" => MemSizeUnits::Mega,
            "get.mem.free.gb" => MemSizeUnits::Giga,
            _ => return Ok(None)
        };
        return kern.lock().drv.mem.free(units).map_err(|e| KernErr::DrvErr(DrvErr::Mem(e))).map(|res| Some((res, ath)))
    }
}

#[async_trait(?Send)]
impl ServHlr for HWHlr {
    async fn hlr(&self, msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
        if let Some((free_mem, ath)) = Self::get_freemem(Rc::new(msg.ath.clone()), msg.msg.clone(), msg.msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::int(free_mem as i32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    }
}
