use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::MemSizeUnits;

use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, UnitModify};


pub const SERV_PATH: &'static str = "io.store";
pub const SERV_HELP: &'static str = "Disk units storage service\nExample: {save:`Some beautiful text` out:@txt.doc}@io.store # save text to `txt.doc` path\n(load @txt.doc)@io.store";


fn get_size(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(usize, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((res, ath)) = read_async!(msg, ath, orig, kern)? {
            // database size
            if let Some(s) = res.as_str() {
                let units = match s.as_str() {
                    "get.size" => MemSizeUnits::Bytes,
                    "get.size.kb" => MemSizeUnits::Kilo,
                    "get.size.mb" => MemSizeUnits::Mega,
                    "get.size.gb" => MemSizeUnits::Giga,
                    _ => return Ok(None)
                };
    
                let size = kern.lock().ram_store.data.size(units);
                return Ok(Some((size, ath)))
            }

            // unit size
            if let Some((s, path)) = msg.as_pair().into_iter().find_map(|(u0, u1)| Some((u0, u1.as_path()?))) {
                if let Some((s, ath)) = read_async!(s, ath, orig, kern)?.and_then(|(s, ath)| Some((s.as_str()?, ath))) {
                    let units = match s.as_str() {
                        "get.size" => MemSizeUnits::Bytes,
                        "get.size.kb" => MemSizeUnits::Kilo,
                        "get.size.mb" => MemSizeUnits::Mega,
                        "get.size.gb" => MemSizeUnits::Giga,
                        _ => return Ok(None)
                    };

                    let size = kern.lock().ram_store.load(Unit::path_share(path)).ok_or(KernErr::DbLoadFault)?.size(units);
                    return Ok(Some((size, ath)))
                }
            }
        }

        Ok(None)
    })
}

fn load(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(Unit, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((s, path)) = msg.as_pair().into_iter().find_map(|(u0, u1)| Some((u0, u1.as_path()?))) {
            if let Some((s, ath)) = read_async!(s, ath, orig, kern)?.and_then(|(s, ath)| Some((s.as_str()?, ath))) {
                if Rc::unwrap_or_clone(s) == "load" {
                    let u = kern.lock().ram_store.load(Unit::path_share(path)).ok_or(KernErr::DbLoadFault)?;
                    return Ok(Some((u, ath)))
                }
            }
        }
        Ok(None)
    })
}

fn save(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Rc<String>, KernErr>> {
    thread!({
        if let Some((u, ath)) = as_map_find_async!(msg, "save", ath, orig, kern)? {
            if let Some(path) = msg.as_map_find("out").and_then(|u| u.as_path()) {
                kern.lock().ram_store.save(Unit::path_share(path), u);
                return Ok(ath)
            }
        }
        Ok(ath)
    })
}

pub fn store_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());

        // get size
        if let Some((size, ath)) = thread_await!(get_size(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let m = Unit::map(&[
                (Unit::str("msg"), Unit::uint(size as u32))]
            );

            let _msg = msg.msg.merge_with(m);
            return kern.lock().msg(&ath, _msg).map(|msg| Some(msg))
        }

        // load
        if let Some((u, ath)) = thread_await!(load(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let m = Unit::map(&[
                (Unit::str("msg"), u)]
            );

            let _msg = msg.msg.merge_with(m);
            return kern.lock().msg(&ath, _msg).map(|msg| Some(msg))
        }

        // save
        let _ath = thread_await!(save(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))?;

        if ath != _ath {
            msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
        }

        Ok(Some(msg))
    })
}