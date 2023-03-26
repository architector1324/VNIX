use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::DrvErr;

use crate::vnix::utils;
use crate::{thread, thread_await, as_map_find_async, as_async, maybe_ok, maybe};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, UnitTypeReadAsync};


pub const SERV_PATH: &'static str = "gfx.2d";
pub const SERV_HELP: &'static str = "Service for rendering 2d graphics\nExample: #ff0000@gfx.2d # fill screen with red color";


fn fill_act(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<((usize, usize), u32)> {
    thread!({
        // #ff0000
        if let Some((col, ath)) = as_async!(msg, as_str, ath, orig, kern)? {
            let col = maybe_ok!(utils::hex_to_u32(&col));
            let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;

            return Ok(Some(((res, col), ath)))
        }

        // ((320 240) #ff0000)
        if let Some(((res, col), ath)) = as_async!(msg, as_pair, ath, orig, kern)? {
            let ((w, h), ath) = maybe!(as_async!(res, as_pair, ath, orig, kern));
            let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
            let (h, ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));

            let (col, ath) = maybe!(as_async!(col, as_str, ath, orig, kern));
            let col = maybe_ok!(utils::hex_to_u32(&col));

            return Ok(Some((((w as usize, h as usize), col), ath)))
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
        if let Some((((w, h), col), ath)) = thread_await!(fill_act(Rc::new(msg.ath.clone()), msg.msg.clone(), msg.msg.clone(), kern))? {
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
