use alloc::vec;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, Schema, SchemaUnit, FromUnit};


pub struct Task {
    task: Option<Unit>
}

impl Default for Task {
    fn default() -> Self {
        Task {
            task: None
        }
    }
}

impl FromUnit for Task {
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut inst = Task::default();

        // config instance
        let mut schm = Schema::Unit(SchemaUnit::Map(vec![(
            Schema::Value(Unit::Str("msg".into())),
            Schema::Unit(SchemaUnit::Unit(&mut inst.task))
        )]));

        schm.find(u);

        Some(inst)
    }
}

impl ServHlr for Task {
    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(u) = &self.task {
            let ath = msg.ath.clone();

            let task = kern.msg(&ath, u.clone())?;
            let msg = kern.task(task)?;

            if let Some(out) = msg.map(|msg| msg.msg.find_unit(&mut vec!["msg".into()].iter())).flatten() {
                let msg = Unit::Map(vec![
                    (Unit::Str("msg".into()), out)
                ]);

                return kern.msg(&ath, msg).map(|msg| Some(msg));
            }

            return Ok(None);
        }

        Ok(Some(msg))
    }
}
