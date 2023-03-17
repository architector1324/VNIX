use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils::Maybe;

use super::msg::Msg;
use super::kern::{KernErr, Kern};
use super::task::ThreadAsync;

use spin::Mutex;


pub type ServHlrAsync<'a> = ThreadAsync<'a, Maybe<Msg, KernErr>>;
pub type ServHlr = dyn Fn(Msg, ServInfo, &Mutex<Kern>) -> ServHlrAsync;


#[derive(Debug)]
pub enum ServErr {
    NotValidUnit
}

#[derive(Debug, Clone)]
pub struct ServInfo {
    pub name: String
}

pub struct Serv {
    pub info: ServInfo,
    pub help: String,
    pub hlr: Box<ServHlr>
}


impl Serv {
    pub fn new(name: &str, help: &str, hlr: Box<ServHlr>) -> Self {
        Serv {
            info: ServInfo {
                name: name.into(),
            },
            help: help.into(),
            hlr
        }
    }
}
