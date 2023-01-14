use alloc::string::String;

use super::msg::Msg;
use super::kern::{KernErr, Kern};


#[derive(Debug)]
pub enum ServErr {
    NotValidUnit
}

pub struct Serv<'a, 'b> {
    pub name: String,
    pub kern: &'b mut Kern<'a>,
}

pub trait ServHlr: Sized + Default {
    fn inst(msg: Msg, serv: &mut Serv) -> Result<(Self, Msg), KernErr>;
    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr>;
}
