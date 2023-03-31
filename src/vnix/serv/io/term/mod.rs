pub mod base;

mod text;
mod media;

use core::fmt::Display;

use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::format;
use alloc::vec::Vec;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::DrvErr;

use crate::vnix::core::task::ThreadAsync;
use crate::vnix::utils::Maybe;
use crate::{thread, thread_await, maybe_ok, read_async, maybe, as_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServInfo, ServHlrAsync};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, UnitModify, UnitReadAsync};


pub const SERV_PATH: &'static str = "io.term";
pub const SERV_HELP: &'static str = "Terminal I/O service\nExample: hello@io.term";

#[derive(Debug, Clone)]
pub enum Mode {
    Text,
    Gfx,
}

impl Display for Mode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Mode::Text => write!(f, "txt"),
            Mode::Gfx => write!(f, "gfx")
        }
    }
}

fn get(ath: Rc<String>, _orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let info = {
            let mode = kern.lock().term.lock().mode.clone();
            let res_txt = kern.lock().drv.cli.res().map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?;
            let res_gfx = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;

            let res_all_txt = kern.lock().drv.cli.res_list()
                .map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?
                .into_iter()
                .map(|(w, h)| Unit::pair(Unit::uint(w as u32), Unit::uint(h as u32)))
                .collect::<Vec<_>>();

            let res_all_gfx = kern.lock().drv.disp.res_list()
                .map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?
                .into_iter()
                .map(|(w, h)| Unit::pair(Unit::uint(w as u32), Unit::uint(h as u32)))
                .collect::<Vec<_>>();

            Unit::map(&[
                (
                    Unit::str("mode"),
                    Unit::str(format!("{mode}").as_str())
                ),
                (
                    Unit::str("res"),
                    Unit::map(&[
                        (
                            Unit::str("txt"),
                            Unit::pair(
                                Unit::uint(res_txt.0 as u32),
                                Unit::uint(res_txt.1 as u32)
                            )
                        ),
                        (
                            Unit::str("gfx"),
                            Unit::pair(
                                Unit::uint(res_gfx.0 as u32),
                                Unit::uint(res_gfx.1 as u32)
                            )
                        ),
                        (
                            Unit::str("all"),
                            Unit::map(&[
                                (
                                    Unit::str("txt"),
                                    Unit::list(&res_all_txt)
                                ),
                                (
                                    Unit::str("gfx"),
                                    Unit::list(&res_all_gfx)
                                )
                            ])
                        )
                    ])
                )
            ])
        };

        yield;

        // get
        if let Some(s) = msg.as_str() {
            let res = match s.as_str() {
                "get" => info,
                "get.mode" => maybe_ok!(info.find(["mode"].into_iter())),
                "get.res" => maybe_ok!(info.find(["res"].into_iter())),
                "get.res.txt" => maybe_ok!(info.find(["res", "txt"].into_iter())),
                "get.res.gfx" => maybe_ok!(info.find(["res", "gfx"].into_iter())),
                "get.res.all" => maybe_ok!(info.find(["res", "all"].into_iter())),
                "get.res.all.txt" => maybe_ok!(info.find(["res", "all", "txt"].into_iter())),
                "get.res.all.gfx" => maybe_ok!(info.find(["res", "all", "gfx"].into_iter())),
                _ => return Ok(None)
            };
            return Ok(Some((res, ath)))
        }
        Ok(None)
    })
}

fn set(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (s, msg) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        return match s.as_str() {
            "set.mode" => {
                let (mode, ath) = maybe!(as_async!(msg, as_str, ath, orig, kern));
                match mode.as_str() {
                    "txt" => kern.lock().term.lock().mode = Mode::Text,
                    "gfx" => kern.lock().term.lock().mode = Mode::Gfx,
                    _ => return Ok(None)
                };
                Ok(Some(ath))
            },
            "set.res.txt" => {
                let ((w, h), ath) = maybe!(as_async!(msg, as_pair, ath, orig, kern));
                let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
                let (h, ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));

                kern.lock().drv.cli.set_res((w as usize, h as usize)).map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?;

                return Ok(Some(ath))
            },
            "set.res.gfx" => {
                let ((w, h), ath) = maybe!(as_async!(msg, as_pair, ath, orig, kern));
                let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
                let (h, ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));

                kern.lock().drv.disp.set_res((w as usize, h as usize)).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;

                return Ok(Some(ath))
            },
            _ => Ok(None)
        }
        
    })
}

pub fn term_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());
        let (_msg, mut ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // get command
        if let Some((msg, ath)) = thread_await!(get(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // set command
        if let Some(_ath) = thread_await!(set(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // cls command
        if let Some(_ath) = thread_await!(text::cls(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // nl command
        if let Some(_ath) = thread_await!(text::nl(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // get key command
        if let Some((key, ath)) = thread_await!(text::get_key(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::str(format!("{key}").as_str()))
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // input command
        if let Some((_msg, ath)) = thread_await!(text::input(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if let Some(msg) = _msg {
                let msg = Unit::map(&[
                    (Unit::str("msg"), msg)
                ]);
                return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
            }
            return Ok(Some(msg))
        }

        // img command
        if let Some(_ath) = thread_await!(media::img(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // say command
        if let Some(_ath) = thread_await!(text::say(false, false, ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }
        Ok(Some(msg))
    })
}
