use core::fmt::{Display, Formatter, write};

use heapless::LinearMap;

use super::unit::Unit;
use super::user::Usr;

#[derive(Debug, Clone)]
pub struct Msg {
    pub msg: LinearMap<Unit, Unit, 1024>,
    pub ath: Usr
}

impl Display for Msg {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write(f, core::format_args!("{{ath:{} msg:{{", self.ath))?;

        for (i, (u0, u1)) in self.msg.iter().enumerate() {
            if i == self.msg.len() - 1 {
                write(f, core::format_args!("{}:{}", u0, u1))?;
            } else {
                write(f, core::format_args!("{}:{} ", u0, u1))?;
            }
        }

        write(f, core::format_args!("}}"))?;

        Ok(())
    }
}
