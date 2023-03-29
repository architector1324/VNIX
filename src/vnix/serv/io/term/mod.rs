pub mod base;

mod text;

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

use crate::{thread, thread_await, as_async, maybe_ok, read_async, maybe};

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

fn get(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
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

        // get
        if let Some((s, ath)) = as_async!(msg, as_str, ath, orig, kern)? {
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
