pub mod base;
pub mod help;

mod text;
mod media;

use core::fmt::Display;

use spin::Mutex;
use async_trait::async_trait;

use alloc::format;
use alloc::vec::Vec;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils::Maybe;
use crate::{maybe_ok, read_async, maybe, as_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::Yield;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::driver::{DrvErr, CLIErr, DispErr};
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};
use crate::vnix::core::unit::{Unit, UnitAs, UnitAsyncResult, UnitModify, UnitNew, UnitReadAsyncI};


pub const SERV_PATH: &'static str = "io.term";

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

pub struct TermHlr;

impl TermHlr {
    async fn get(ath: Rc<String>, _orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitAsyncResult {
        let s = maybe_ok!(msg.as_str());

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

            let res_min_txt = kern.lock().drv.cli.res_list().map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?.into_iter().min().ok_or(KernErr::DrvErr(DrvErr::CLI(CLIErr::GetResolution)))?;
            let res_max_txt = kern.lock().drv.cli.res_list().map_err(|e| KernErr::DrvErr(DrvErr::CLI(e)))?.into_iter().max().ok_or(KernErr::DrvErr(DrvErr::CLI(CLIErr::GetResolution)))?;

            let res_min_gfx = kern.lock().drv.disp.res_list().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?.into_iter().min().ok_or(KernErr::DrvErr(DrvErr::Disp(DispErr::GetResolution)))?;
            let res_max_gfx = kern.lock().drv.disp.res_list().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?.into_iter().max().ok_or(KernErr::DrvErr(DrvErr::Disp(DispErr::GetResolution)))?;

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
                            Unit::str("min"),
                            Unit::map(&[
                                (
                                    Unit::str("txt"),
                                    Unit::pair(
                                        Unit::uint(res_min_txt.0 as u32),
                                        Unit::uint(res_min_txt.1 as u32)
                                    )
                                ),
                                (
                                    Unit::str("gfx"),
                                    Unit::pair(
                                        Unit::uint(res_min_gfx.0 as u32),
                                        Unit::uint(res_min_gfx.1 as u32)
                                    )
                                ),
                            ])
                        ),
                        (
                            Unit::str("max"),
                            Unit::map(&[
                                (
                                    Unit::str("txt"),
                                    Unit::pair(
                                        Unit::uint(res_max_txt.0 as u32),
                                        Unit::uint(res_max_txt.1 as u32)
                                    )
                                ),
                                (
                                    Unit::str("gfx"),
                                    Unit::pair(
                                        Unit::uint(res_max_gfx.0 as u32),
                                        Unit::uint(res_max_gfx.1 as u32)
                                    )
                                ),
                            ])
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

        Yield::now().await;

        // get
        let res = match s.as_str() {
            "get" => info,
            "get.mode" => maybe_ok!(info.find(["mode"].into_iter())),
            "get.res" => maybe_ok!(info.find(["res"].into_iter())),
            "get.res.txt" => maybe_ok!(info.find(["res", "txt"].into_iter())),
            "get.res.gfx" => maybe_ok!(info.find(["res", "gfx"].into_iter())),
            "get.res.min" => maybe_ok!(info.find(["res", "min"].into_iter())),
            "get.res.min.txt" => maybe_ok!(info.find(["res", "min", "txt"].into_iter())),
            "get.res.min.gfx" => maybe_ok!(info.find(["res", "min", "gfx"].into_iter())),

            "get.res.max" => maybe_ok!(info.find(["res", "max"].into_iter())),
            "get.res.max.txt" => maybe_ok!(info.find(["res", "max", "txt"].into_iter())),
            "get.res.max.gfx" => maybe_ok!(info.find(["res", "max", "gfx"].into_iter())),
            "get.res.all" => maybe_ok!(info.find(["res", "all"].into_iter())),
            "get.res.all.txt" => maybe_ok!(info.find(["res", "all", "txt"].into_iter())),
            "get.res.all.gfx" => maybe_ok!(info.find(["res", "all", "gfx"].into_iter())),
            _ => return Ok(None)
        };
        return Ok(Some((res, ath)))
    }
    
    async fn set(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> Maybe<Rc<String>, KernErr> {
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
                let (res, ath) = maybe!(read_async!(msg, ath, orig, kern));
                let (w, h) = if let Some((w, h)) = res.clone().as_pair() {
                    (w, h)
                } else if let Some(s) = res.as_str() {
                    match s.as_str() {
                        "240p" => (Unit::uint(426), Unit::uint(240)),
                        "360p" => (Unit::uint(640), Unit::uint(360)),
                        "720p" | "hd" => (Unit::uint(1280), Unit::uint(720)),
                        "1080p" | "fhd" => (Unit::uint(1920), Unit::uint(1080)),
                        "2k" | "qhd" => (Unit::uint(2560), Unit::uint(1440)),
                        "4k" | "uhd" => (Unit::uint(3840), Unit::uint(2160)),
                        "8k" => (Unit::uint(7680), Unit::uint(4320)),
                        _ => return Ok(None)
                    }
                } else {
                    return Ok(None)
                };

                let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
                let (h, ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));

                kern.lock().drv.disp.set_res((w as usize, h as usize)).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;

                return Ok(Some(ath))
            },
            _ => Ok(None)
        }
    }    
}

#[async_trait(?Send)]
impl ServHlr for TermHlr {
    async fn hlr(&self, mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
        let ath = Rc::new(msg.ath.clone());
        let (_msg, mut ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // get command
        if let Some((msg, ath)) = Self::get(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // set command
        if let Some(_ath) = Self::set(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // cls command
        if let Some(_ath) = text::Text::cls(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // nl command
        if let Some(_ath) = text::Text::nl(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // get key command
        if let Some((key, ath)) = text::Text::get_key(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::str(format!("{key}").as_str()))
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // input command
        if let Some((_msg, ath)) = text::Text::input(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if let Some(msg) = _msg {
                let msg = Unit::map(&[
                    (Unit::str("msg"), msg)
                ]);
                return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
            }
            return Ok(Some(msg))
        }

        // image command
        if let Some((_, _ath)) = media::Media::img((0, 0), ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // video command
        if let Some(_ath) = media::Media::vid((0, 0), ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // sprite command
        if let Some(_ath) = media::Media::spr(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }

        // say command
        if let Some(_ath) = text::Text::say(false, false, None, None, ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if _ath != ath {
                ath = _ath;
                msg = kern.lock().msg(&ath, _msg)?;
            }
            return Ok(Some(msg))
        }
        Ok(Some(msg))
    }
}
