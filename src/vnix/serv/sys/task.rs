use alloc::vec;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaUnit, Schema};


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

        let schm = SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit);
        inst.task = schm.find(u);

        Some(inst)
    }
}

impl ServHlr for Task {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Service for run task from message\nExample: {msg:{msg:hello task:io.term} task:sys.task}".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(u) = &self.task {
            let ath = msg.ath.clone();

            let task = kern.msg(&ath, u.clone())?;
            let msg = kern.task(task)?;

            let schm = SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit);

            if let Some(out) = msg.map(|msg| schm.find(&msg.msg)).flatten() {
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
