use super::msg::Msg;
use super::kern::{KernErr, Kern};


pub trait Serv: Sized + Default {
    fn inst(msg: Msg, kern: &mut Kern) -> Result<(Self, Msg), KernErr>;
    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr>;
}
