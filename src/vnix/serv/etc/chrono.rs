use alloc::vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;


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

impl ServHlr for Chrono {
    fn inst(msg: Msg, serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = Chrono::default();

        // config instance
        msg.msg.find_int(&mut vec!["wait".into()].iter()).map(|mcs| inst.wait.replace(mcs as usize));

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some(mcs) = self.wait {
            serv.kern.time.wait(mcs).map_err(|e| KernErr::TimeErr(e))?;
        }
        Ok(Some(msg))
    }
}
