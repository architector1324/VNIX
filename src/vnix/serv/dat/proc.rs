use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::{thread, thread_await, as_async, maybe, read_async, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitTypeReadAsync, UnitNew};


pub const SERV_PATH: &'static str = "dat.proc";
pub const SERV_HELP: &'static str = "Common data processing service\nExample: (len [1 2 3])@dat.proc # count list length";

fn len(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<usize> {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));
        
        if s.as_str() != "len" {
            return Ok(None)
        }
        
        let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

        // string length
        if let Some(s) = dat.clone().as_str() {
            let len = s.chars().count();
            return Ok(Some((len, ath)))
        }

        // list length
        if let Some(lst) = dat.as_list() {
            let len = lst.len();
            return Ok(Some((len, ath)))
        }

        Ok(None)
    })
}

fn keys(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Vec<Unit>> {
    thread!({
        let (s, map) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "keys" {
            return Ok(None)
        }

        let (map, ath) = maybe!(as_async!(map, as_map, ath, orig, kern));
        let keys = map.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>();

        Ok(Some((keys, ath)))
    })
}

pub fn proc_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // len
        if let Some((len, ath)) = thread_await!(len(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(len as u32))
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // keys
        if let Some((keys, ath)) = thread_await!(keys(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::list(&keys))
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }
        Ok(Some(msg))
    })
}
