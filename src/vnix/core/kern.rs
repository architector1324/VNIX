use core::ops::Deref;

use heapless::pool::{Pool, Box};
use heapless::{Vec, LinearMap};

use super::msg::Msg;
use super::serv::Serv;
use super::unit::Unit;
use super::unit::UnitParseErr;

use super::user::Usr;

use crate::vnix::serv::io;

use crate::driver::CLI;
use crate::driver::CLIErr;


#[derive(Debug)]
pub enum KernErr {
    MemoryOut,
    EncodeFault,
    UsrNotFound,
    ServNotFound,
    ParseErr(UnitParseErr),
    CLIErr(CLIErr)
}

pub struct Kern<'a> {
    // drivers
    pub cli: &'a mut dyn CLI,

    // vnix
    units: Pool<Unit>,
    users: Vec<Usr, 32>
}


impl<'a> Kern<'a> {
    pub fn new(cli: &'a mut dyn CLI) -> Self {
        let kern = Kern {
            cli,
            units: Pool::new(),
            users: Vec::new(),
        };

        static mut UNITS_MEM: [u8; 256 * core::mem::size_of::<Unit>()] = [0; 256 * core::mem::size_of::<Unit>()];

        unsafe {
            kern.units.grow(&mut UNITS_MEM);
        }

        kern
    }

    pub fn reg_usr(&mut self, usr: Usr) -> Result<(), KernErr> {
        self.users.push(usr).map_err(|_| KernErr::MemoryOut)
    }

    pub fn msg(&self, ath: &str, u: Box<Unit>) -> Result<Msg, KernErr> {
        let usr = self.users.iter().find(|usr| usr.name == ath).ok_or(KernErr::UsrNotFound).cloned()?;
        Msg::new(usr, u)
    }

    pub fn send(&mut self, serv: &str, msg: Msg) -> Result<Option<Msg>, KernErr> {
        match serv {
            "io.term" => {
                let (inst, msg) = io::Term::inst(msg, self)?;
                inst.handle(msg, self)
            },
            _ => Err(KernErr::ServNotFound)
        }
    }

    pub fn unit(&mut self, u: Unit) -> Result<Box<Unit>, KernErr> {
        if let Some(b) = self.units.alloc() {
           return Ok(b.init(u));
        }
        Err(KernErr::MemoryOut)
    }

    pub fn dup(&mut self, u: &Box<Unit>) -> Result<Box<Unit>, KernErr> {
        if let Some(b) = self.units.alloc() {
            let n_u = match u.deref() {
                Unit::None => Unit::None,
                Unit::Bool(v) => Unit::Bool(*v),
                Unit::Byte(v) => Unit::Byte(*v),
                Unit::Int(v) => Unit::Int(*v),
                Unit::Dec(v) => Unit::Dec(*v),
                Unit::Str(s) => Unit::Str(s.as_str().into()),
                Unit::Pair(p) => Unit::Pair((self.dup(&p.0)?, self.dup(&p.1)?)),
                Unit::Lst(lst) => {
                    let mut n_lst = Vec::new();

                    for u in lst {
                        n_lst.push(self.dup(u)?).map_err(|_| KernErr::MemoryOut)?;
                    }

                    Unit::Lst(n_lst)
                },
                Unit::Map(m) => {
                    let mut n_map = LinearMap::new();

                    for (u0, u1) in m {
                        n_map.insert(self.dup(u0)?, self.dup(u1)?).map_err(|_| KernErr::MemoryOut)?;
                    }
                    Unit::Map(n_map)
                }
            };
            return Ok(b.init(n_u));
        }
        Err(KernErr::MemoryOut)
    }
}
