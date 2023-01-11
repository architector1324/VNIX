use core::ops::Deref;

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
        if let Unit::Map(ref m) = msg.msg {
            let mut it = m.iter().filter_map(|p| Some((p.0.as_str()?, p.1.as_int()?)));
            it.find(|(s, _)| s == "wait").map(|(_, mcs)| inst.wait.replace(mcs as usize));
        }

        Ok((inst, msg))
    }

    fn handle(&self, _msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(mcs) = self.wait {
            kern.time.wait(mcs).map_err(|e| KernErr::TimeErr(e))?;
        }
        Ok(None)
    }
}
