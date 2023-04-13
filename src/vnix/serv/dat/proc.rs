use core::pin::Pin;
use core::cmp::Ordering;
use core::ops::{Generator, GeneratorState};

use alloc::borrow::ToOwned;
use sha3::{Digest, Sha3_256};
use base64ct::{Base64, Encoding};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::format;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils;
use crate::{thread, thread_await, as_async, maybe, read_async, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::driver::MemSizeUnits;
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

fn cat(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        match s.as_str() {
            "cat" => {
                let ((a, b), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));
                let (a, ath) = maybe!(read_async!(a, ath, orig, kern));
                let (b, ath) = maybe!(read_async!(b, ath, orig, kern));

                // (<str> <str>)
                if let Some((a, b)) = a.clone().as_str().and_then(|a| Some((a, b.clone().as_str()?))) {
                    let s = Rc::unwrap_or_clone(a).as_str().to_owned() + Rc::unwrap_or_clone(b).as_str();
                    return Ok(Some((Unit::str(&s), ath)))
                }

                // (<list> <list>)
                if let Some((a, b)) = a.clone().as_list().and_then(|a| Some((a, b.clone().as_list()?))) {
                    let lst = a.iter().cloned().chain(b.iter().cloned()).collect::<Vec<_>>();
                    return Ok(Some((Unit::list(&lst), ath)))
                }

                // (<unit> <list>)
                if let Some(b) = b.clone().as_list() {
                    let lst = core::iter::once(a).chain(b.iter().cloned()).collect::<Vec<_>>();
                    return Ok(Some((Unit::list(&lst), ath)))
                }

                // (<list> <unit>)
                if let Some(a) = a.as_list() {
                    let lst = a.iter().cloned().chain(core::iter::once(b)).collect::<Vec<_>>();
                    return Ok(Some((Unit::list(&lst), ath)))
                }
                Ok(None)
            },
            "cat.zip" => {
                let ((a, b), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));
                let (a, ath) = maybe!(as_async!(a, as_list, ath, orig, kern));
                let (b, ath) = maybe!(as_async!(b, as_list, ath, orig, kern));

                let lst = if a.len() == b.len() {
                    let a_it = a.iter().cloned();
                    let b_it = b.iter().cloned();

                    a_it.zip(b_it).map(|(a, b)| Unit::pair(a, b)).collect::<Vec<_>>()
                } else {
                    let max_len = a.len().max(b.len());
                    
                    let a_it = a.iter().chain(a.iter().cycle().take(max_len - a.len())).cloned();
                    let b_it = b.iter().chain(b.iter().cycle().take(max_len - b.len())).cloned();

                    a_it.zip(b_it).map(|(a, b)| Unit::pair(a, b)).collect::<Vec<_>>()
                };
                Ok(Some((Unit::list(&lst), ath)))
            },
            _ => Ok(None)
        }
    })
}

fn split(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        match s.as_str() {
            "split" => todo!(),
            "split.uz" => {
                let (lst, mut ath) = maybe!(as_async!(dat, as_list, ath, orig, kern));

                let mut lst0 = Vec::with_capacity(lst.len());
                let mut lst1 = Vec::with_capacity(lst.len());

                for p in Rc::unwrap_or_clone(lst) {
                    let ((a, b), _ath) = maybe!(as_async!(p, as_pair, ath, orig, kern));
                    lst0.push(a);
                    lst1.push(b);
                    ath = _ath;
                }

                let res = Unit::pair(Unit::list(&lst0), Unit::list(&lst1));
                Ok(Some((res, ath)))
            }
            _ => Ok(None)
        }
    })
}

fn map(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, stream) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "map" {
            return Ok(None)
        }

        let (dat, serv, _) = maybe_ok!(stream.as_stream());
        let (com, lst) = maybe_ok!(dat.as_pair());

        let (com, ath) = maybe!(read_async!(com, ath, orig, kern));
        let (lst, mut ath) = maybe!(read_async!(lst, ath, orig, kern));
        
        let serv = Rc::new(serv);

        if let Some(lst) = lst.clone().as_list() {
            let streams = lst.iter().cloned().map(|u| Unit::stream_loc(Unit::pair(com.clone(), u), &serv)).collect::<Vec<_>>();

            let mut lst = Vec::new();
            for u in streams {
                let (u, _ath) = maybe!(read_async!(u, ath, orig, kern));
                lst.push(u);
                ath = _ath;
            }
            return Ok(Some((Unit::list(&lst), ath)))
        } else if let Some((a, b)) = lst.as_pair() {
            let stream = Unit::stream_loc(Unit::pair(com.clone(), a), &serv);
            let (a, ath) = maybe!(read_async!(stream, ath, orig, kern));

            let stream = Unit::stream_loc(Unit::pair(com, b), &serv);
            let (b, ath) = maybe!(read_async!(stream, ath, orig, kern));

            return Ok(Some((Unit::pair(a, b), ath)))
        }
        Ok(None)
    })
}

fn fold(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, stream) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "fold" {
            return Ok(None)
        }

        let (dat, serv, _) = maybe_ok!(stream.as_stream());
        let (com, lst) = maybe_ok!(dat.as_pair());

        let (com, ath) = maybe!(read_async!(com, ath, orig, kern));
        let (lst, mut ath) = maybe!(as_async!(lst, as_list, ath, orig, kern));

        let serv = Rc::new(serv);
        let mut it = Rc::unwrap_or_clone(lst).into_iter();

        let mut res = maybe_ok!(it.next());

        for u in it {
            let u = Unit::pair(com.clone(), Unit::pair(res, u));
            let stream = Unit::stream_loc(u, &serv);

            let (_res, _ath) = maybe!(read_async!(stream, ath, orig, kern));

            ath = _ath;
            res = _res;
        }

        Ok(Some((res, ath)))
    })
}

fn dup(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "dup" {
            return Ok(None)
        }

        let ((cnt, u), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));
        let (cnt, ath) = maybe!(as_async!(cnt, as_uint, ath, orig, kern));
        let (u, ath) = maybe!(read_async!(u, ath, orig, kern));

        let lst = (0..cnt).map(|_| u.clone()).collect::<Vec<_>>();
        Ok(Some((Unit::list_share(Rc::new(lst)), ath)))
    })
}

fn make(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "make" {
            return Ok(None)
        }

        let ((into, from), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));
        let (into, ath) = maybe!(as_async!(into, as_str, ath, orig, kern));

        let (u, ath) = match into.as_str() {
            "pair" => {
                let (lst, ath) = maybe!(as_async!(from, as_list, ath, orig, kern));
                if lst.len() != 2 {
                    return Ok(None)
                }
                (Unit::pair(lst[0].clone(), lst[1].clone()), ath)
            },
            "lst" => {
                let (dat, ath) = maybe!(read_async!(from, ath, orig, kern));

                if let Some((u0, u1)) = dat.clone().as_pair() {
                    (Unit::list(&[u0, u1]), ath)
                } else if let Some(map) = dat.as_map() {
                    let lst = map.iter().cloned().map(|(u0, u1)| Unit::pair(u0, u1)).collect::<Vec<_>>();
                    (Unit::list(&lst), ath)
                } else {
                    return Ok(None)
                }
            },
            "map" => {
                let (dat, mut ath) = maybe!(read_async!(from, ath, orig, kern));

                if let Some((u0, u1)) = dat.clone().as_pair() {
                    (Unit::map(&[(u0, u1)]), ath)
                } else if let Some(lst) = dat.as_list() {
                    let mut map = Vec::with_capacity(lst.len());
                    for u in Rc::unwrap_or_clone(lst) {
                        let ((u0, u1), _ath) = maybe!(as_async!(u, as_pair, ath, orig, kern));
                        map.push((u0, u1));
                        ath = _ath;
                    }
                    (Unit::map(&map), ath)
                } else {
                    return Ok(None)
                }
            }
            _ => return Ok(None)
        };
        Ok(Some((u, ath)))
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
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "get" {
            return Ok(None)
        }

        let ((path, src), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));

        let path = maybe_ok!(path.as_path());
        let (src, ath) = maybe!(read_async!(src, ath, orig, kern));

        let u = maybe_ok!(src.find(path.iter().map(|s| s.as_str())));
        Ok(Some((u, ath)))
    })
}

fn first_last(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        let (u, ath) = match s.as_str() {
            "fst" => {
                let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

                let u = if let Some(lst) = dat.clone().as_list() {
                    maybe_ok!(lst.get(0).cloned())
                } else if let Some((a, _)) = dat.as_pair() {
                    a
                } else {
                    return Ok(None)
                };
                (u, ath)
            },
            "last" => {
                let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));

                let u = if let Some(lst) = dat.clone().as_list() {
                    maybe_ok!(lst.iter().last().cloned())
                } else if let Some((_, b)) = dat.as_pair() {
                    b
                } else {
                    return Ok(None)
                };
                (u, ath)
            },
            _ => return Ok(None)
        };
        Ok(Some((u, ath)))
    })
}

fn take(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Vec<Unit>> {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        if s.as_str() != "take" {
            return Ok(None)
        }

        let ((count, lst), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));
        let (count, ath) = maybe!(as_async!(count, as_uint, ath, orig, kern));
        let (lst, ath) = maybe!(as_async!(lst, as_list, ath, orig, kern));

        let res = lst.iter().take(count as usize).cloned().collect::<Vec<_>>();
        Ok(Some((res, ath)))
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

fn size(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<usize> {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        let units = match s.as_str() {
            "size" => MemSizeUnits::Bytes,
            "size.kb" => MemSizeUnits::Kilo,
            "size.mb" => MemSizeUnits::Mega,
            "size.gb" => MemSizeUnits::Giga,
            _ => return Ok(None)
        };

        let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));
        let size = dat.size(units);

        Ok(Some((size, ath)))
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
            "dser.str" => {
                let (s, ath) = maybe!(as_async!(dat, as_str, ath, orig, kern));
                let u = Unit::parse(s.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
                (u, ath)
            },
            "ser.bytes" => {
                let (dat, ath) = maybe!(read_async!(dat, ath, orig, kern));
                let b = dat.as_bytes().into_iter().map(|b| Unit::byte(b)).collect::<Vec<_>>();
                (Unit::list(&b), ath)
            },
            "dser.bytes" => {
                let (dat, mut ath) = maybe!(as_async!(dat, as_list, ath, orig, kern));
                let mut lst = Vec::with_capacity(dat.len());
                for u in Rc::unwrap_or_clone(dat) {
                    let (u, _ath) = maybe!(as_async!(u, as_byte, ath, orig, kern));
                    lst.push(u);
                    ath = _ath;
                }
                let u = Unit::parse(lst.iter()).map_err(|e| KernErr::ParseErr(e))?.0;
                (u, ath)
            },
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

        // map
        if let Some((msg, ath)) = thread_await!(map(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // reduce
        if let Some((msg, ath)) = thread_await!(fold(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // reduce
        if let Some((msg, ath)) = thread_await!(dup(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // concatenate
        if let Some((msg, ath)) = thread_await!(cat(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // split
        if let Some((msg, ath)) = thread_await!(split(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // make
        if let Some((msg, ath)) = thread_await!(make(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
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

        // take
        if let Some((res, ath)) = thread_await!(take(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::list(&res))
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

        // first/last
        if let Some((msg, ath)) = thread_await!(first_last(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
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

        // size
        if let Some((size, ath)) = thread_await!(size(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(size as u32))
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
