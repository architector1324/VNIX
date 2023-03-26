use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::MemSizeUnits;

use crate::vnix::utils::Maybe;
use crate::{thread, thread_await, read_async, as_map_find_async, as_async, maybe, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, UnitTypeReadAsync, UnitReadAsync};


pub const SERV_PATH: &'static str = "io.store";
pub const SERV_HELP: &'static str = "Disk units storage service\nExample: {save:`Some beautiful text` out:@txt.doc}@io.store # save text to `txt.doc` path\n(load @txt.doc)@io.store";


fn get_size(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<usize> {
    thread!({
        let (s, u, ath) = if let Some(s) = msg.clone().as_str() {
            // database
            (s, kern.lock().ram_store.data.clone(), ath)
        } else if let Some((u, path)) = msg.as_pair().into_iter().find_map(|(u0, u1)| Some((u0, u1.as_path()?))) {
            let (s, ath) = maybe!(as_async!(u, as_str, ath, orig, kern));
            // unit
            (s, kern.lock().ram_store.load(Unit::path_share(path)).ok_or(KernErr::DbLoadFault)?, ath)
        } else {
            return Ok(None);
        };

        let units = match s.as_str() {
            "get.size" => MemSizeUnits::Bytes,
            "get.size.kb" => MemSizeUnits::Kilo,
            "get.size.mb" => MemSizeUnits::Mega,
            "get.size.gb" => MemSizeUnits::Giga,
            _ => return Ok(None)
        };

        let size = u.size(units);
        Ok(Some((size, ath)))
    })
}

fn load(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (u, path) = maybe_ok!(msg.as_pair().into_iter().find_map(|(u0, u1)| Some((u0, u1.as_path()?))));
        let (s, ath) = maybe!(as_async!(u, as_str, ath, orig, kern));

        if s.as_str() == "load" {
            let u = kern.lock().ram_store.load(Unit::path_share(path)).ok_or(KernErr::DbLoadFault)?;
            return Ok(Some((u, ath)))
        }
        Ok(None)
    })
}

fn save(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (u, ath) = maybe!(as_map_find_async!(msg, "save", ath, orig, kern));
        let path = maybe_ok!(msg.as_map_find("out").and_then(|u| u.as_path()));

        kern.lock().ram_store.save(Unit::path_share(path), u);
        Ok(Some(ath))
    })
}

pub fn store_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // get size
        if let Some((size, ath)) = thread_await!(get_size(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(size as u32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // load
        if let Some((u, ath)) = thread_await!(load(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), u)]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // save
        if let Some(_ath) = thread_await!(save(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), _msg)?;
            }
        }

        Ok(Some(msg))
    })
}