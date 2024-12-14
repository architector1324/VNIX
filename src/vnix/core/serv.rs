use async_trait::async_trait;

use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils::Maybe;

use super::msg::Msg;
use super::kern::{KernErr, Kern};

use spin::Mutex;


// pub type ServHlrAsync<'a> = ThreadAsync<'a, Maybe<Msg, KernErr>>;
// pub type ServHlr = Box<dyn Fn(Msg, ServInfo, &Mutex<Kern>) -> ServHlrAsync>;

#[async_trait(?Send)]
pub trait ServHlr {
    async fn help_hlr(&self, msg: Msg, serv: ServInfo, kern: &Mutex<Kern>) -> Maybe<Msg, KernErr>;
    async fn hlr(&self, msg: Msg, serv: ServInfo, kern: &Mutex<Kern>) -> Maybe<Msg, KernErr>;
}

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
    pub hlr: Box<dyn ServHlr>
}


impl Serv {
    pub fn new(name: &str, hlr: Box<dyn ServHlr>) -> Self {
        Serv {
            info: ServInfo {
                name: name.into(),
            },
            hlr
        }
    }
}
