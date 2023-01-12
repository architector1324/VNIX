use alloc::vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::Serv;
use crate::vnix::core::kern::{KernErr, Kern};


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

impl Serv for Chrono {
    fn inst(msg: Msg, _kern: &mut Kern) -> Result<(Self, Msg), KernErr> {
        let mut inst = Chrono::default();

        // config instance
        msg.msg.find_int(&mut vec!["wait".into()].iter()).map(|mcs| inst.wait.replace(mcs as usize));

        Ok((inst, msg))
    }

    fn handle(&self, _msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(mcs) = self.wait {
            kern.time.wait(mcs).map_err(|e| KernErr::TimeErr(e))?;
        }
        Ok(None)
    }
}
