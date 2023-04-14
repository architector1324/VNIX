use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::driver::DrvErr;

use crate::vnix::utils;
use crate::{thread, thread_await, as_async, maybe_ok, maybe, read_async, as_map_find_as_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitParse, UnitModify, UnitReadAsyncI, UnitTypeReadAsync};


pub const SERV_PATH: &'static str = "gfx.2d";

fn fill_act(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<((usize, usize), u32)> {
    thread!({
        // #ff0000
        if let Some(col) = msg.clone().as_str() {
            let col = maybe_ok!(utils::hex_to_u32(&col));
            let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;

            return Ok(Some(((res, col), ath)))
        }

        // (fill #ff0000)
        if let Some((s, col)) = msg.clone().as_pair() {
            let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

            if s.as_str() != "fill" {
                return Ok(None)
            }

            let (col, ath) = maybe!(as_async!(col, as_str, ath, orig, kern));
            let col = maybe_ok!(utils::hex_to_u32(&col));

            let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;

            return Ok(Some(((res, col), ath)))
        }

        // {fill:#ff0000} | {fill:((320 240) #ff0000)}
        if let Some((col, mut ath)) = as_map_find_as_async!(msg, "fill", as_str, ath, orig, kern)? {
            let col = maybe_ok!(utils::hex_to_u32(&col));

            let res = if let Some(((w, h), _ath)) = as_map_find_as_async!(msg, "size", as_pair, ath, orig, kern)? {
                let (w, _ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
                let (h, _ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));

                ath = _ath;
                (w as usize, h as usize)
            } else {
                kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?
            };

            return Ok(Some(((res, col), ath)))
        }
        Ok(None)
    })
}

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let s = maybe_ok!(msg.msg.clone().as_str());

        let help_s = "{
            name:gfx.2d
            info:`Service for rendering 2d graphics to image, create video from image sequence, apply filters, effects etc.`
            tut:[
                {
                    info:`Create image filled some color.`
                    com:[
                        #ff0000@gfx.2d
                        (fill #ff0000)@gfx.2d
                        {
                            fill:#ff0000
                        }@gfx.2d
                    ]
                    res:{
                        size:(1280 800)
                        fmt:rgb.rle
                        img:[(1024000 16711680)]
                    }
                }
                {
                    info:`Create image with specified size with filled some color.`
                    com:{
                        fill:#ff0000
                        size:(320 240)
                    }@gfx.2d
                    res:{
                        size:(320 240)
                        fmt:rgb.rle
                        img:[(76800 16711680)]
                    }
                }
            ]
            man:{
                fill:{
                    info:`Create image with specified size with filled some color.`
                    schm:[
                        `str: #<r8><g8><b8>`
                        (fill `str: #<r8><g8><b8>`)
                        {
                            fill:`str: #<r8><g8><b8>`
                            size:(uint uint)
                        }
                    ]
                    tut:[
                        @tut.0
                        @tut.1
                    ]
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

pub fn gfx2d_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath, msg.msg.clone(), kern));

        if let Some((((w, h), col), ath)) = thread_await!(fill_act(ath.clone(), _msg.clone(), _msg, kern))? {
            let msg = Unit::map(&[
                (
                    Unit::str("msg"),
                    Unit::map(&[
                        (
                            Unit::str("size"),
                            Unit::pair(
                                Unit::uint(w as u32),
                                Unit::uint(h as u32)
                            )
                        ),
                        (
                            Unit::str("fmt"),
                            Unit::str("rgb.rle")
                        ),
                        (
                            Unit::str("img"),
                            Unit::list(&[
                                Unit::pair(
                                    Unit::uint((w * h) as u32),
                                    Unit::uint(col as u32)
                                )
                            ])
                        )
                    ])
                ),
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg));
        }
        Ok(Some(msg))
    })
}
