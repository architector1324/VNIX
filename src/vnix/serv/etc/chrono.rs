use alloc::vec;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaInt, Schema};


#[derive(Debug)]
pub struct Chrono {
    wait: Option<i32>
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

        let schm = SchemaMapEntry(Unit::Str("wait".into()), SchemaInt);
        inst.wait = schm.find_loc(u);

        Some(inst)
    }
}

impl ServHlr for Chrono {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Service for time control\nExample: {wait:1000000 task:etc.chrono} # wait for 1 sec.".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(mcs) = self.wait {
            kern.time.wait(mcs as usize).map_err(|e| KernErr::TimeErr(e))?;
        }
        Ok(Some(msg))
    }
}
