use core::ops::Generator;

use alloc::boxed::Box;
use alloc::string::String;

use super::msg::Msg;
use super::kern::{KernErr, Kern};
use super::unit::Unit;

use spin::Mutex;


pub enum ServHelpTopic {
    Info
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

pub type ServHlrAsync<'a> = Box<dyn Generator<Yield = (), Return = Result<Option<Msg>, KernErr>> + 'a>;

pub trait ServHlr {
    fn inst(&self, msg: &Unit) -> Result<Box<dyn ServHlr>, KernErr>;

    fn help<'a>(self: Box<Self>, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a>;
    fn handle<'a>(self: Box<Self>, msg: Msg, serv: ServInfo, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a>;
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
