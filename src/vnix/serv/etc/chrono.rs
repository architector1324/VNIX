use alloc::vec;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;
use crate::vnix::core::unit::{Schema, SchemaUnit, Unit};


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

impl ServHlr for Chrono {
    fn inst(msg: Msg, _serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = Chrono::default();

        // config instance
        let mut schm = Schema::Unit(
            SchemaUnit::Map(vec![(
                Schema::Value(Unit::Str("wait".into())),
                Schema::Unit(SchemaUnit::Int(&mut inst.wait))
            )])
        );

        schm.find(&msg.msg);

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some(mcs) = self.wait {
            serv.kern.time.wait(mcs as usize).map_err(|e| KernErr::TimeErr(e))?;
        }
        Ok(Some(msg))
    }
}
