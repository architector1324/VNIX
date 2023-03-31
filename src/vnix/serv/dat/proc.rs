use core::pin::Pin;
use core::cmp::Ordering;
use core::ops::{Generator, GeneratorState};

use sha3::{Digest, Sha3_256};
use base64ct::{Base64, Encoding};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::format;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils;
use crate::{thread, thread_await, as_async, maybe, read_async, maybe_ok, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitTypeReadAsync, UnitNew, UnitAsBytes, UnitReadAsync, UnitParse, UnitModify};


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

fn sort(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "sort" {
            return Ok(None)
        }

        let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

        // (a b)
        if let Some((a, b)) = dat.clone().as_pair() {
            let u = match maybe_ok!(a.partial_cmp(&b)) {
                Ordering::Greater => Unit::pair(b, a),
                _ => dat
            };
            return Ok(Some((u, ath)))
        }

        // [v0 ..]
        if let Some(lst) = dat.as_list() {
            let mut lst = Rc::unwrap_or_clone(lst);
            lst.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));

            return Ok(Some((Unit::list(&lst), ath)))
        }
        Ok(None)
    })
}

fn rev(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "rev" {
            return Ok(None)
        }

        let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

        // (a b)
        if let Some((a, b)) = dat.clone().as_pair() {
            return Ok(Some((Unit::pair(b, a), ath)))
        }

        // [v0 ..]
        if let Some(lst) = dat.as_list() {
            let mut lst = Rc::unwrap_or_clone(lst);
            lst.reverse();

            return Ok(Some((Unit::list(&lst), ath)))
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

fn get(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let path = maybe_ok!(msg.clone().as_map_find("get").and_then(|u| u.as_path()));
        let (from, ath) = maybe!(as_map_find_async!(msg, "from", ath, orig, kern));

        let u = maybe_ok!(from.find(path.iter().map(|s| s.as_str())));
        Ok(Some((u, ath)))
    })
}

fn zip(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Rc<String>> {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "zip" {
            return Ok(None)
        }

        let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

        let b = dat.as_bytes();
        let s = utils::compress_bytes(&b)?;

        return Ok(Some((Rc::new(s), ath)))
    })
}

fn unzip(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat_s) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "unzip" {
            return Ok(None)
        }

        let (s, ath) = maybe!(as_async!(dat_s, as_str, ath, orig, kern));

        let dat = utils::decompress_bytes(&s)?;
        let msg = Unit::parse(dat.iter()).map_err(|e| KernErr::ParseErr(e))?.0;

        Ok(Some((msg, ath)))
    })
}

fn hash(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Rc<String>> {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "hash" {
            return Ok(None)
        }

        let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

        let h = Sha3_256::digest(dat.as_bytes());
        let s = Base64::encode_string(&h[..]);

        return Ok(Some((Rc::new(s), ath)))
    })
}

fn serialize(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        let (u, ath) = match s.as_str() {
            "ser.str" => {
                let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));
                let s = format!("{dat}");
                (Unit::str(&s), ath)
            },
            "ser.bytes" => {
                let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));
                let b = dat.as_bytes().into_iter().map(|b| Unit::byte(b)).collect::<Vec<_>>();
                (Unit::list(&b), ath)
            }
            _ => return Ok(None)
        };
        return Ok(Some((u, ath)))
    })
}

fn enumerate(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "enum" {
            return Ok(None)
        }

        let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

        // (a b)
        if let Some((a, b)) = dat.clone().as_pair() {
            let u = Unit::list(&[
                Unit::pair(Unit::uint(0), a),
                Unit::pair(Unit::uint(1), b)
            ]);
            return Ok(Some((u, ath)))
        }

        // [v0 ..]
        if let Some(lst) = dat.as_list() {
            let lst = lst.iter().enumerate().map(|(i, u)| Unit::pair(Unit::uint(i as u32), u.clone())).collect::<Vec<_>>();
            return Ok(Some((Unit::list(&lst), ath)))
        }
        Ok(None)
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

        // sort
        if let Some((msg, ath)) = thread_await!(sort(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // rev
        if let Some((msg, ath)) = thread_await!(rev(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // enumerate
        if let Some((msg, ath)) = thread_await!(enumerate(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
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

        // get
        if let Some((msg, ath)) = thread_await!(get(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // zip
        if let Some((s, ath)) = thread_await!(zip(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::str_share(s))
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // unzip
        if let Some((msg, ath)) = thread_await!(unzip(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // hash
        if let Some((s, ath)) = thread_await!(hash(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::str_share(s))
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // serialize
        if let Some((msg, ath)) = thread_await!(serialize(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }
        Ok(Some(msg))
    })
}
