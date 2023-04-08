use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::format;
use alloc::vec::Vec;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils::Maybe;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::driver::{TermKey, DrvErr};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, DisplayStr, DisplayShort, UnitTypeReadAsync};

use crate::{thread, thread_await, as_async, maybe, read_async, as_map_find_as_async, maybe_ok};

use super::base;


pub fn cls(ath: Rc<String>, _orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let s = maybe_ok!(msg.as_str());
        if s.as_str() != "cls" {
            return Ok(None)
        }

        let term = kern.lock().term.clone();

        term.lock().clear(&mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;
        term.lock().flush(&mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;

        yield;
        Ok(Some(ath))
    })
}

pub fn nl(ath: Rc<String>, _orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let s = maybe_ok!(msg.as_str());
        if s.as_str() != "nl" {
            return Ok(None)
        }

        let term = kern.lock().term.clone();
        term.lock().print_ch('\n', &mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;

        yield;
        Ok(Some(ath))
    })
}

pub fn say(nl: bool, fmt: bool, shrt: Option<usize>, mut ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        if let Some((s, msg)) = msg.clone().as_pair() {
            if let Some((s, ath)) = as_async!(s, as_str, ath, orig, kern)? {
                match s.as_str() {
                    // (say <unit>)
                    "say" => return thread_await!(say(false, false, None, ath, orig, msg, kern)),
                    // (say.fmt [<unit> ..])
                    "say.fmt" => return thread_await!(say(false, true, None, ath, orig, msg, kern)),
                    _ => ()
                }
            }
        }

        // {say:<unit> nl:<t|f> shrt:<uint>}
        if let Some(_msg) = msg.clone().as_map_find("say") {
            let nl = if let Some((nl, _ath)) = as_map_find_as_async!(msg, "nl", as_bool, ath, orig, kern)? {
                ath = _ath;
                nl
            } else {
                false
            };

            let shrt = if let Some((shrt, _ath)) = as_map_find_as_async!(msg, "shrt", as_uint, ath, orig, kern)? {
                ath = _ath;
                Some(shrt as usize)
            } else {
                None
            };

            return thread_await!(say(nl, false, shrt, ath, orig, _msg, kern))
        }

        // {say.fmt:[<unit> ..] nl:<t|f> shrt:<uint>}
        if let Some((lst, mut ath)) = as_map_find_as_async!(msg, "say.fmt", as_list, ath, orig, kern)? {
            let nl = if let Some((nl, _ath)) = as_map_find_as_async!(msg, "nl", as_bool, ath, orig, kern)? {
                ath = _ath;
                nl
            } else {
                false
            };

            let shrt = if let Some((shrt, _ath)) = as_map_find_as_async!(msg, "shrt", as_uint, ath, orig, kern)? {
                ath = _ath;
                Some(shrt as usize)
            } else {
                None
            };

            return thread_await!(say(nl, true, shrt, ath, orig, Unit::list_share(lst), kern))
        }

        // <unit>
        let mut s = if fmt {
            let (lst, _ath) = maybe!(as_async!(msg, as_list, ath, orig, kern));
            ath = _ath; 

            let mut out = Vec::new();

            for u in Rc::unwrap_or_clone(lst) {
                let (u, _ath) = maybe!(read_async!(u, ath, orig, kern));

                let s = match shrt {
                    Some(shrt) => format!("{}", DisplayShort(shrt, u)),
                    None => format!("{}", DisplayStr(u))
                };
                out.push(s);
                ath = _ath;
            }
            out.join("")
        } else {
            if let Some((msg, _ath)) = read_async!(msg, ath, orig, kern)? {
                ath = _ath;

                match shrt {
                    Some(shrt) => format!("{}", DisplayShort(shrt, msg)),
                    None => format!("{}", DisplayStr(msg))
                }
            } else {
                return Ok(Some(ath))
            }
        };

        if nl {
            s += "\n";
        }

        let term = kern.lock().term.clone();
        term.lock().print(s.as_str(), &mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;

        Ok(Some(ath))
    })
}

pub fn get_key(ath: Rc<String>, _orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<TermKey> {
    thread!({
        let s = maybe_ok!(msg.as_str());

        match s.as_str() {
            "inp.key" => {
                let key = loop {
                    if let Some(key) = kern.lock().drv.cli.get_key(false).map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))? {
                        break key;
                    }
                    yield;
                };
                Ok(Some((key, ath)))
            },
            "inp.key.async" => {
                let key = maybe!(kern.lock().drv.cli.get_key(false).map_err(|e| KernErr::DrvErr(DrvErr::CLI(e))));
                Ok(Some((key, ath)))
            },
            _ => Ok(None)
        }
    })
}

pub fn input(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Option<Unit>> {
    thread!({
        let term = kern.lock().term.clone();

        // inp
        if let Some(s) = msg.clone().as_str() {
            return match s.as_str() {
                "inp" => {
                    let inp = base::Term::input(term, false, false, None, kern);
                    let res = thread_await!(inp)?;
                    Ok(Some((res, ath)))
                },
                _ => Ok(None)
            }
        }

        // (inp <pmt>)
        if let Some((s, pmt)) = msg.clone().as_pair() {
            let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));
            
            return match s.as_str() {
                "inp" => {
                    let (pmt, ath) = maybe!(as_async!(pmt, as_str, ath, orig, kern));
                    term.lock().print(&pmt, &mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;

                    let inp = base::Term::input(term, false, false, None, kern);
                    let res = thread_await!(inp)?;
                    Ok(Some((res, ath)))
                },
                _ => Ok(None)
            }
        }

        // {inp:<pmt> prs:<t|f> nl:<t|f>}
        if let Some((pmt, mut ath)) = as_map_find_as_async!(msg, "inp", as_str, ath, orig, kern)? {
            let prs = if let Some((prs, _ath)) = as_map_find_as_async!(msg, "prs", as_bool, ath, orig, kern)? {
                ath = _ath;
                prs
            } else {
                false
            };

            let nl = if let Some((nl, _ath)) = as_map_find_as_async!(msg, "nl", as_bool, ath, orig, kern)? {
                ath = _ath;
                nl
            } else {
                false
            };

            let sct = if let Some((sct, _ath)) = as_map_find_as_async!(msg, "sct", as_bool, ath, orig, kern)? {
                ath = _ath;
                sct
            } else {
                false
            };

            let lim = if let Some((lim, _ath)) = as_map_find_as_async!(msg, "lim", as_uint, ath, orig, kern)? {
                ath = _ath;
                Some(lim as usize)
            } else {
                None
            };

            term.lock().print(&pmt, &mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;

            let inp = base::Term::input(term.clone(), sct, prs, lim, kern);
            let res = thread_await!(inp)?;

            if nl {
                term.lock().print_ch('\n', &mut kern.lock()).map_err(|e| KernErr::DrvErr(e))?;
            }

            return Ok(Some((res, ath)))
        }

        Ok(None)
    })
}