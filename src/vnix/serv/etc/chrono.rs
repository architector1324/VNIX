use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use spin::Mutex;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{ServHlr, ServHelpTopic, ServHlrAsync, ServInfo};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaInt, Schema, SchemaOr, Or};


#[derive(Debug)]
pub struct Chrono {
    wait: Option<usize>
}

impl Default for Chrono {
    fn default() -> Self {
        Chrono {
            wait: None
        }
    }
}

impl FromUnit for Chrono {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = Chrono::default();

        let schm = SchemaOr(
            SchemaInt,
            SchemaMapEntry(Unit::Str("wait".into()), SchemaInt)
        );
        inst.wait = schm.find_loc(u).map(|or| {
            match or {
                Or::First(mcs) => mcs as usize,
                Or::Second(mcs) => mcs as usize
            }
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
        let hlr = move || {
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Service for time control\nExample: {wait:1000000}@etc.chrono # wait for 1 sec.".into())
            };
    
            let m = Unit::Map(vec![(
                Unit::Str("msg".into()),
                u
            )]);
    
            let out = kern.lock().msg(&ath, m).map(|msg| Some(msg));
            yield;

            out
        };
        Box::new(hlr)
    }

    fn handle<'a>(self: Box<Self>, msg: Msg, _serv: ServInfo, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            if let Some(mcs) = self.wait {
                kern.lock().drv.time.wait(mcs).map_err(|e| KernErr::TimeErr(e))?;
                yield;
            }
            Ok(Some(msg))
        };
        Box::new(hlr)
    }
}
