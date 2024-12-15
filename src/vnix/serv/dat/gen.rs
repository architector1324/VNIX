use alloc::rc::Rc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use spin::Mutex;
use async_trait::async_trait;

use crate::{as_async, maybe, read_async, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::Kern;
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitNew, UnitAsyncResult};


pub const SERV_PATH: &'static str = "dat.gen";

pub const SERV_HELP: &'static str = "{
    name:dat.gen
    info:`Common data generation service`
    tut:[
        {
            info:`Generate list with integers sequence`
            com:(lin.int (1 5))@dat.gen
            res:[1 2 3 4 5]
        }
        {
            info:`Generate list with bytes sequence`
            com:(lin.byte (0x01 0x04))@dat.gen
            res:[0x01 0x02 0x03 0x04]
        }
        {
            info:`Generate random integer`
            com:(rnd.int (1 5))@dat.gen
            res:3
        }
        {
            info:`Generate random byte`
            com:(rnd.byte (0x1a 0xff))@dat.gen
            res:0x2c
        }
    ]
    man:{
        lin:{
            info:`Generate list with data sequence`
            schm:[
                (lin.int (int int))
                (lin.byte (byte byte))
            ]
            tut:[@tut.0 @tut.1]
        }
        rnd:{
            info:`Generate random data`
            schm:[
                (rnd.int (int int))
                (rnd.byte (byte byte))
            ]
            tut:[@tut.2 @tut.3]
        }
    }
}";

pub struct GenHlr;

impl GenHlr {
    async fn lin(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitAsyncResult {
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
    }
}

#[async_trait(?Send)]
impl ServHlr for GenHlr {
    async fn hlr(&self, msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // lin
        if let Some((msg, ath)) = Self::lin(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)
            ]);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    }
}
