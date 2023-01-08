use super::msg::Msg;
use super::kern::{KernErr, Kern};


pub trait Serv {
    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr>;
}
