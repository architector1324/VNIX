use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::driver::{DrvErr, MemSizeUnits};

use crate::{thread, thread_await, maybe, maybe_ok, as_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitNew, UnitModify, UnitAs, UnitParse, UnitTypeReadAsync};


pub const SERV_PATH: &'static str = "sys.hw";

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

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let s = maybe_ok!(msg.msg.clone().as_str());

        let help_s = "{
            name:sys.hw
            info:`Service for hardware management`
            tut:[
                {
                    info:`Get free RAM space`
                    com:[
                        get.mem.free@sys.hw
                        get.mem.free.kb@sys.hw
                        get.mem.free.mb@sys.hw
                        get.mem.free.gb@sys.hw
                    ]
                    res:512
                }
            ]
            man:{
                get.mem.free:{
                    info:`Get free RAM space`
                    tut:@tut.0
                }
            }
        }";
        let help = Unit::parse(help_s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
        yield;

        let res = match s.as_str() {
            "help" => help,
            "help.name" => maybe_ok!(help.find(["name"].into_iter())),
            "help.info" => maybe_ok!(help.find(["info"].into_iter())),
            "help.tut" => maybe_ok!(help.find(["tut"].into_iter())),
            "help.man" => maybe_ok!(help.find(["man"].into_iter())),
            _ => return Ok(None)
        };

        let _msg = Unit::map(&[
            (Unit::str("msg"), res)
        ]);
        kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
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
