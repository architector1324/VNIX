use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::vec;
use alloc::boxed::Box;
use alloc::string::String;

use spin::Mutex;

use crate::{thread, thread_await};
use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{ServHlr, ServHelpTopic, ServHlrAsync, ServInfo};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaInt, Schema, SchemaOr, Or};
use crate::driver::Duration;


#[derive(Debug)]
struct Wait {
    dur: Duration,
}

#[derive(Debug)]
enum Act {
    Wait(Wait)
}

#[derive(Debug)]
pub struct Chrono {
    act: Option<Act>
}

impl Default for Chrono {
    fn default() -> Self {
        Chrono {
            act: None
        }
    }
}

impl FromUnit for Chrono {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = Chrono::default();

        let schm = SchemaOr(
            SchemaInt,
            SchemaOr(
                SchemaMapEntry(Unit::Str("wait".into()), SchemaInt),
                SchemaOr(
                    SchemaMapEntry(Unit::Str("wait.ms".into()), SchemaInt),
                    SchemaMapEntry(Unit::Str("wait.mcs".into()), SchemaInt)
                )
            )
        );

        inst.act = schm.find_loc(u).map(|or| {
            let dur = match or {
                Or::First(sec) => Duration::Seconds(sec as usize),
                Or::Second(or) =>
                    match or {
                        Or::First(sec) => Duration::Seconds(sec as usize),
                        Or::Second(or) =>
                            match or {
                                Or::First(ms) => Duration::Milli(ms as usize),
                                Or::Second(mcs) => Duration::Micro(mcs as usize),
                            }
                    }
            };

            Act::Wait(Wait {
                dur
            })
        });

        Some(inst)
    }
}

impl ServHlr for Chrono {
    fn inst(&self, msg: &Unit) -> Result<Box<dyn ServHlr>, KernErr> {
        let inst = Self::from_unit_loc(msg).ok_or(KernErr::CannotCreateServInstance)?;
        Ok(Box::new(inst))
    }

    fn help<'a>(self: Box<Self>, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        thread!({
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Service for time control\nExample: {wait.ms:500}@time.chrono # wait for 0.5 sec.".into())
            };
    
            let m = Unit::Map(vec![(
                Unit::Str("msg".into()),
                u
            )]);
    
            let out = kern.lock().msg(&ath, m).map(|msg| Some(msg));
            yield;
    
            out
        })
    }

    fn handle<'a>(self: Box<Self>, msg: Msg, _serv: ServInfo, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        thread!({
            if let Some(act) = self.act {
                match act {
                    Act::Wait(wait) => {
                        let wait = kern.lock().drv.time.wait_async(wait.dur);
                        thread_await!(wait).map_err(|e| KernErr::TimeErr(e))?;
                    }
                }
            }
            Ok(Some(msg))
        })
    }
}
