use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::DrvErr;

use crate::vnix::utils;
use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI};


pub const SERV_PATH: &'static str = "gfx.2d";
pub const SERV_HELP: &'static str = "Service for rendering 2d graphics\nExample: #ff0000@gfx.2d # fill screen with red color";


fn fill_act(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<((usize, usize), u32, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((res, ath)) = read_async!(msg, ath, orig, kern)? {
            // #ff0000
            if let Some(col) = res.as_str().and_then(|s| utils::hex_to_u32(&s)) {
                let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
                return Ok(Some((res, col, ath)))
            }

            // ((320 240) #ff0000)
            if let Some(((w, h), col)) = msg.clone().as_pair().into_iter().filter_map(|(u0, u1)| Some((u0.as_pair()?, u1))).next() {
                if let Some((w, ath)) = read_async!(w, ath, orig, kern)?.and_then(|(v, ath)| Some((v.as_int()?, ath))) {
                    if let Some((h, ath)) = read_async!(h, ath, orig, kern)?.and_then(|(v, ath)| Some((v.as_int()?, ath))) {
                        if let Some((col, ath)) = read_async!(col, ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_str()?, ath))).and_then(|(s, ath)| Some((utils::hex_to_u32(&s)?, ath))) {
                            return Ok(Some(((w as usize, h as usize), col, ath)))
                        }
                    }
                }
            }
        }

        if let Some((msg, ath)) = as_map_find_async!(msg, "fill", ath, orig, kern)? {
            // {fill:#ff0000} | {fill:((320 240) #ff0000)}
            return thread_await!(fill_act(ath, orig, msg, kern));
        }
        Ok(None)
    })
}

pub fn gfx2d_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        if let Some(((w, h), col, ath)) = thread_await!(fill_act(Rc::new(msg.ath.clone()), msg.msg.clone(), msg.msg.clone(), kern))? {
            let m = Unit::map(&[
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

            return kern.lock().msg(&ath, m).map(|msg| Some(msg));
        }
        Ok(Some(msg))
    })
}
