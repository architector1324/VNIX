use spin::Mutex;

use alloc::vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::msg::Msg;

use crate::driver::MemSizeUnits;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic, ServHlrAsync};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, Schema, SchemaOr, Or, SchemaStr};


#[derive(Debug, Clone)]
struct FreeMem(MemSizeUnits);

#[derive(Debug, Clone)]
enum Act {
    FreeMem(FreeMem),
}

pub struct HW {
    act: Option<Act>
}

impl Default for HW {
    fn default() -> Self {
        HW {
            act: None
        }
    }
}

impl FromUnit for FreeMem {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaMapEntry(Unit::Str("hw".into()), SchemaStr),
            SchemaStr
        );

        schm.find_deep(glob, u).and_then(|or| {
            let s = match or {
                Or::First(s) => s,
                Or::Second(s) => s
            };

            match s.as_str() {
                "get.mem.free" => Some(FreeMem(MemSizeUnits::Bytes)),
                "get.mem.free.kb" => Some(FreeMem(MemSizeUnits::Kilo)),
                "get.mem.free.mb" => Some(FreeMem(MemSizeUnits::Mega)),
                "get.mem.free.gb" => Some(FreeMem(MemSizeUnits::Giga)),
                _ => None
            }
        })
    }
}

impl FromUnit for HW {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut hw = HW::default();

        if let Some(free_mem) = FreeMem::from_unit(u, u) {
            hw.act = Some(Act::FreeMem(free_mem));
            return Some(hw)
        }

        Some(hw)
    }
}

impl ServHlr for HW {
    fn help<'a>(self, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Service for hardware management\nExample: get.mem.free.mb@sys.hw".into())
            };

            let m = Unit::Map(vec![(
                Unit::Str("msg".into()),
                u
            )]);
    
            let out = kern.lock().msg(&ath, m).map(|msg| Some(msg));
            yield;

            out
        };
        ServHlrAsync(Box::new(hlr))
    }

    fn handle<'a>(self, msg: Msg, _serv: Serv, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            if let Some(act) = self.act {
                let u = match act {
                    Act::FreeMem(free_mem) => Unit::Int(kern.lock().drv.mem.free(free_mem.0).map_err(|e| KernErr::MemErr(e))? as i32)
                };
                yield;

                let m = Unit::Map(vec![
                    (Unit::Str("msg".into()), u)]
                );

                let _msg = msg.msg.merge(m);
                kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
            } else {
                Ok(Some(msg))
            }
        };
        ServHlrAsync(Box::new(hlr))
    }
}