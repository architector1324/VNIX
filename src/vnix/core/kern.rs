use alloc::vec;
use alloc::vec::Vec;

use super::msg::Msg;
use super::serv::{Serv, ServHlr};
use super::serv::ServErr;
use super::unit::Unit;
use super::unit::UnitParseErr;

use super::user::Usr;

use crate::vnix::serv::{io, etc, gfx, math, sys};

use crate::driver::{CLIErr, DispErr, TimeErr, RndErr, CLI, Disp, Time, Rnd};

#[derive(Debug)]
pub enum KernErr {
    MemoryOut,
    EncodeFault,
    DecodeFault,
    CompressionFault,
    DecompressionFault,
    CreatePrivKeyFault,
    CreatePubKeyFault,
    SignFault,
    SignVerifyFault,
    UsrNotFound,
    UsrNameAlreadyReg,
    UsrAlreadyReg,
    UsrRegWithAnotherName,
    ServNotFound,
    ParseErr(UnitParseErr),
    CLIErr(CLIErr),
    DispErr(DispErr),
    TimeErr(TimeErr),
    RndErr(RndErr),
    ServErr(ServErr)
}

pub struct Kern<'a> {
    // drivers
    pub cli: &'a mut dyn CLI,
    pub disp: &'a mut dyn Disp,
    pub time: &'a mut dyn Time,
    pub rnd: &'a mut dyn Rnd,

    // vnix
    users: Vec<Usr>
}

impl<'a> Kern<'a> {
    pub fn new(cli: &'a mut dyn CLI, disp: &'a mut dyn Disp, time: &'a mut dyn Time, rnd: &'a mut dyn Rnd) -> Self {
        let kern = Kern {
            cli,
            disp,
            time,
            rnd,
            users: Vec::new(),
        };

        kern
    }

    pub fn reg_usr(&mut self, usr: Usr) -> Result<(), KernErr> {
        if self.users.iter().find(|u| u.name == usr.name && u.pub_key != usr.pub_key).is_some() {
            return Err(KernErr::UsrNameAlreadyReg);
        }

        if self.users.iter().find(|u| u.name == usr.name && u.pub_key == usr.pub_key).is_some() {
            return Err(KernErr::UsrAlreadyReg);
        }

        if self.users.iter().find(|u| u.name != usr.name && u.pub_key == usr.pub_key).is_some() {
            return Err(KernErr::UsrRegWithAnotherName);
        }

        self.users.push(usr);
        Ok(())
    }

    fn get_usr(&self, ath: &str) -> Result<Usr, KernErr> {
        self.users.iter().find(|usr| usr.name == ath).ok_or(KernErr::UsrNotFound).cloned()
    }

    pub fn msg(&self, ath: &str, u: Unit) -> Result<Msg, KernErr> {
        let usr = self.get_usr(ath)?;
        Msg::new(usr, u)
    }

    fn msg_hlr(msg: Msg, usr: Usr) -> Result<Msg, KernErr> {
        if let Some(_msg) = msg.msg.find_unit(&mut vec!["mrg".into()].iter()) {
            return msg.merge(usr, _msg)
        }
        Ok(msg)
    }

    pub fn task(&mut self, msg: Msg) -> Result<Option<Msg>, KernErr> {
        let path = vec!["task".into()];

        if let Some(serv) = msg.msg.find_str(&mut path.iter()) {
            return self.send(serv.as_str(), msg);
        }

        if let Some(lst) = msg.msg.find_list(&mut path.iter()) {
            let net = lst.iter().filter_map(|u| u.as_str()).collect::<Vec<_>>();

            if net.is_empty() {
                return Ok(None);
            }

            let mut msg = msg;

            loop {
                for (i, serv) in net.iter().enumerate() {
                    if net.len() > 1 && i == net.len() - 1 && net.first().unwrap() == net.last().unwrap() {
                        break;
                    }

                    let u = msg.msg.clone();
    
                    if let Some(mut _msg) = self.send(serv.as_str(), msg)? {
                        let usr = self.get_usr(&_msg.ath)?;
                        msg = _msg.merge(usr, u)?;
                    } else {
                        return Ok(None);
                    }

                    if i == net.len() - 1 && net.first().unwrap() != net.last().unwrap() {
                        return Ok(Some(msg));
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn send<'b>(&'b mut self, serv: &str, msg: Msg) -> Result<Option<Msg>, KernErr> {
        let usr = self.get_usr(&msg.ath)?;
        usr.verify(&msg.msg, &msg.sign)?;

        let msg = Kern::msg_hlr(msg, usr)?;

        match serv {
            "io.term" => {
                let mut serv = Serv {
                    name: "io.term".into(),
                    kern: self,
                };
                let (inst, msg) = io::Term::inst(msg, &mut serv)?;
                inst.handle(msg, &mut serv)
            },
            "etc.chrono" => {
                let mut serv = Serv {
                    name: "etc.chrono".into(),
                    kern: self,
                };
                let (inst, msg) = etc::chrono::Chrono::inst(msg, &mut serv)?;
                inst.handle(msg, &mut serv)
            },
            "etc.fsm" => {
                let mut serv = Serv {
                    name: "etc.fsm".into(),
                    kern: self,
                };
                let (inst, msg) = etc::fsm::FSM::inst(msg, &mut serv)?;
                inst.handle(msg, &mut serv)
            },
            "gfx.2d" => {
                let mut serv = Serv {
                    name: "gfx.2d".into(),
                    kern: self,
                };
                let (inst, msg) = gfx::GFX2D::inst(msg, &mut serv)?;
                inst.handle(msg, &mut serv)
            },
            "math.int" => {
                let mut serv = Serv {
                    name: "math.int".into(),
                    kern: self
                };
                let (inst, msg) = math::Int::inst(msg, &mut serv)?;
                inst.handle(msg, &mut serv)
            },
            "sys.task" => {
                let mut serv = Serv {
                    name: "sys.task".into(),
                    kern: self
                };
                let (inst, msg) = sys::task::Task::inst(msg, &mut serv)?;
                inst.handle(msg, &mut serv)
            },
            "sys.usr" => {
                let mut serv = Serv {
                    name: "sys.usr".into(),
                    kern: self
                };
                let (inst, msg) = sys::usr::User::inst(msg, &mut serv)?;
                inst.handle(msg, &mut serv)
            },
            _ => Err(KernErr::ServNotFound)
        }
    }
}
