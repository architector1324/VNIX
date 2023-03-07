use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use spin::Mutex;
use alloc::boxed::Box;

use crate::driver::DrvErr;

use crate::{thread, thread_await, read_async, as_map_find_async};
use crate::vnix::utils;

use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, SchemaPair, SchemaUnit, Schema};
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "gfx.2d";
pub const SERV_HELP: &'static str = "Service for rendering 2d graphics\nExample: #ff0000@gfx.2d # fill screen with red color";


fn fill_act(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<((usize, usize), u32)>, KernErr>> {
    thread!({
        if let Some(res) = read_async!(msg, ath, orig, kern)? {
            // #ff0000
            if let Some(col) = res.as_str().and_then(|s| utils::hex_to_u32(&s)) {
                let res = kern.lock().drv.disp.res().map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
                return Ok(Some((res, col)))
            }

            // ((320 240) #ff0000)
            let schm = SchemaPair(
                SchemaPair(SchemaUnit, SchemaUnit),
                SchemaUnit
            );

            if let Some(((w, h), col)) = schm.find(&orig, &msg) {
                let w = read_async!(Rc::new(w), ath, orig, kern)?.and_then(|v| v.as_int());
                let h = read_async!(Rc::new(h), ath, orig, kern)?.and_then(|v| v.as_int());
                let col = read_async!(Rc::new(col), ath, orig, kern)?.and_then(|u| u.as_str()).and_then(|s| utils::hex_to_u32(&s));

                if let Some((res, col)) = w.and_then(|w| Some(((w as usize, h? as usize), col?))) {
                    return Ok(Some((res, col)))
                }
            }
        }

        if let Some(msg) = as_map_find_async!(msg, "fill", ath, orig, kern)? {
            // {fill:#ff0000} | {fill:((320 240) #ff0000)}
            return thread_await!(fill_act(ath, orig, Rc::new(msg), kern));
        }
        Ok(None)
    })
}

pub fn gfx2d_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());

        if let Some(((w, h), col)) = thread_await!(fill_act(Rc::new(msg.ath.clone()), u.clone(), u, kern))? {
            let m = Unit::Map(vec![
                (
                    Unit::Str("msg".into()),
                    Unit::Map(vec![
                        (
                            Unit::Str("size".into()),
                            Unit::Pair(
                                Box::new(Unit::Int(w as i32)),
                                Box::new(Unit::Int(h as i32))
                            )
                        ),
                        (
                            Unit::Str("fmt".into()),
                            Unit::Str("rgb.rle".into())
                        ),
                        (
                            Unit::Str("img".into()),
                            Unit::Lst(vec![
                                Unit::Pair(
                                    Box::new(Unit::Int((w * h) as i32)),
                                    Box::new(Unit::Int(col as i32))
                                )
                            ])
                        )
                    ])
                ),
            ]);

            return kern.lock().msg(&msg.ath, m).map(|msg| Some(msg));
        }
        Ok(Some(msg))
    })
}
