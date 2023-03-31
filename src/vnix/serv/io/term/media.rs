use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::format;
use alloc::vec::Vec;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::{TermKey, DrvErr};
use crate::vnix::utils::Maybe;
use crate::vnix::core::task::ThreadAsync;

use crate::{thread, thread_await, as_async, maybe, read_async, as_map_find_as_async, as_map_find_async, maybe_ok};

use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitReadAsyncI, DisplayStr, UnitTypeReadAsync};

use super::base;


struct Img {

}

use core::fmt::Write;

pub fn img(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (dat, ath) = if let Some((dat, ath)) = as_map_find_as_async!(msg, "img", as_list, ath, orig, kern)? {
            (dat, ath)
        } else if let Some((s, ath)) = as_map_find_as_async!(msg, "img", as_str, ath, orig, kern)? {
            // compressed image
            writeln!(kern.lock(), "IMG: {:?}", s);
            return Ok(Some(ath)) // remove me
        } else {
            return Ok(None)
        };

        writeln!(kern.lock(), "IMG: {:?}", dat);

        Ok(Some(ath))
    })
}
