use core::fmt::{Display, Formatter, write};

use super::unit::Unit;
use super::user::Usr;

#[derive(Debug)]
pub enum MsgParseErr {
    NotUnit
}

#[derive(Debug, Clone)]
pub struct Msg<'a> {
    pub msg: Unit<'a>,
    pub ath: Usr
}

impl<'a> Display for Msg<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write(f, core::format_args!("{{ath:{} msg:{}}}", self.ath, self.msg))
    }
}
