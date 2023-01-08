use heapless::pool::Box;
use core::fmt::{Display, Formatter};

use super::unit::Unit;
use super::user::Usr;

#[derive(Debug)]
pub struct Msg {
    pub msg: Box<Unit>,
    pub ath: Usr
}

impl Display for Msg {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ath:{} msg:{}}}", self.ath, self.msg)
    }
}
