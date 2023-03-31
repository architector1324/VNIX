use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::vec::Vec;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::DrvErr;
use crate::vnix::utils::Maybe;
use crate::vnix::core::task::ThreadAsync;

use crate::{thread, thread_await, as_async, maybe, as_map_find_as_async};

use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::unit::{Unit, UnitAs, UnitReadAsyncI};


pub fn img(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        if let super::Mode::Text = kern.lock().term.lock().mode {
            return Ok(None)
        }

        // parse
        let (dat, ath) = maybe!(as_map_find_as_async!(msg, "img", as_list, ath, orig, kern));
        let (fmt, ath) = maybe!(as_map_find_as_async!(msg, "fmt", as_str, ath, orig, kern));

        let ((w, h), ath) = maybe!(as_map_find_as_async!(msg, "size", as_pair, ath, orig, kern));
        let (w, ath) = maybe!(as_async!(w, as_uint, ath, orig, kern));
        let (h, mut ath) = maybe!(as_async!(h, as_uint, ath, orig, kern));

        // get image
        let img = match fmt.as_str() {
            "rgb" => {
                let mut img = Vec::with_capacity((w * h) as usize);
                for px in Rc::unwrap_or_clone(dat) {
                    let (px, _ath) = maybe!(as_async!(px, as_uint, ath, orig, kern));
                    ath = _ath;

                    img.push(px);
                }
                img
            },
            "rgb.rle" => {
                let mut img = Vec::with_capacity((w * h) as usize);
                for px in Rc::unwrap_or_clone(dat) {
                    let ((cnt, px), _ath) = maybe!(as_async!(px, as_pair, ath, orig, kern));
                    let (cnt, _ath) = maybe!(as_async!(cnt, as_uint, _ath, orig, kern));
                    let (px, _ath) = maybe!(as_async!(px, as_uint, _ath, orig, kern));
                    ath = _ath;

                    img.extend((0..cnt).map(|_| px));
                }
                img
            },
            "rgba" => todo!(),
            "rgba.rle" => todo!(),
            _ => return Ok(None)
        };

        // draw
        kern.lock().drv.disp.blk((0, 0), (w as usize, h as usize), 0, &img).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
        kern.lock().drv.disp.flush_blk((0, 0), (w as usize, h as usize)).map_err(|e| KernErr::DrvErr(DrvErr::Disp(e)))?;
        return Ok(Some(ath))
    })
}
