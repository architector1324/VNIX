use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::{thread, thread_await, as_async, maybe, read_async, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitNew, UnitReadAsync};


pub const SERV_PATH: &'static str = "dat.gen";
pub const SERV_HELP: &'static str = "Common data generation service\nExample: (lin.int (1 5))@dat.gen # generate list [1 2 3 4 5]";

fn lin(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        let (s, dat) = maybe_ok!(msg.as_pair());
        let (s, ath) = maybe!(as_async!(s, as_str, ath, orig, kern));

        let (u, ath) = match s.as_str() {
            "lin.int" => {
                let ((start, end), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));
                let (start, ath) = maybe!(as_async!(start, as_int, ath, orig, kern));
                let (end, ath) = maybe!(as_async!(end, as_int, ath, orig, kern));

                let lst = if start <= end {
                    (start..=end).map(|v| Unit::int(v)).collect::<Vec<_>>()
                } else {
                    (end..=start).map(|v| Unit::int(v)).rev().collect::<Vec<_>>()
                };

                (Unit::list_share(Rc::new(lst)), ath)
            },
            "lin.byte" => {
                let ((start, end), ath) = maybe!(as_async!(dat, as_pair, ath, orig, kern));
                let (start, ath) = maybe!(as_async!(start, as_byte, ath, orig, kern));
                let (end, ath) = maybe!(as_async!(end, as_byte, ath, orig, kern));

                let lst = if start <= end {
                    (start..=end).map(|v| Unit::byte(v)).collect::<Vec<_>>()
                } else {
                    (end..=start).map(|v| Unit::byte(v)).rev().collect::<Vec<_>>()
                };

                (Unit::list_share(Rc::new(lst)), ath)
            },
            _ => return Ok(None)
        };
        Ok(Some((u, ath)))
    })
}

pub fn gen_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // lin
        if let Some((msg, ath)) = thread_await!(lin(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}