use alloc::vec;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;
use crate::vnix::core::unit::Unit;


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

impl ServHlr for Task {
    fn inst(msg: Msg, _serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = Task::default();

        // config instance
        msg.msg.find_unit(&mut vec!["msg".into()].iter()).map(|u| inst.task.replace(u));

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some(u) = &self.task {
            let task = serv.kern.msg(&msg.ath, u.clone())?;
            serv.kern.task(task)?;
        }

        Ok(Some(msg))
    }
}