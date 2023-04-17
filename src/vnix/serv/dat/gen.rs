use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;
use alloc::rc::Rc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::{thread, thread_await, as_async, maybe, read_async, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitAs, UnitNew, UnitParse, UnitModify, UnitReadAsync};


pub const SERV_PATH: &'static str = "dat.gen";
const SERV_HELP: &'static str = "{
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

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let s = maybe_ok!(msg.msg.clone().as_str());
        let help = Unit::parse(SERV_HELP.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
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