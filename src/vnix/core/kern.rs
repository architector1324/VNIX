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
        self.users.push(usr);
        Ok(())
    }

    pub fn msg(&self, ath: &str, u: Unit) -> Result<Msg, KernErr> {
        let usr = self.users.iter().find(|usr| usr.name == ath).ok_or(KernErr::UsrNotFound).cloned()?;
        Msg::new(usr, u)
    }

    pub fn task(&mut self, msg: Msg) -> Result<Option<Msg>, KernErr> {
        let usr = self.users.iter().find(|usr| usr.name == msg.ath).ok_or(KernErr::UsrNotFound).cloned()?;
        let path = vec!["task".into()];

        if let Some(serv) = msg.msg.find_str(&mut path.iter()) {
            return self.send(serv.as_str(), msg);
        }

        if let Some(lst) = msg.msg.find_list(&mut path.iter()) {
            let net = lst.iter().filter_map(|u| u.as_str()).collect::<Vec<_>>();
            let merge = msg.msg.find_bool(&mut vec!["mrg".into()].iter()).unwrap_or(false);

            if net.is_empty() {
                return Ok(None);
            }

            let mut msg = msg;
            let u = msg.msg.clone();

            if let Some(mut _msg) = self.send(net.first().unwrap().as_str(), msg)? {
                if merge {
                    let msg_merge = _msg.msg.find_map(&mut vec!["msg".into()].iter());

                    if let Some(m) = msg_merge {
                        _msg = _msg.merge(usr.clone(), Unit::Map(m))?;
                    }
                }
                msg = _msg.merge(usr.clone(), u)?;
            } else {
                return Ok(None);
            }

            loop {
                for (i, serv) in net.iter().skip(1).enumerate() {
                    let u = msg.msg.clone();
    
                    if let Some(mut _msg) = self.send(serv.as_str(), msg)? {
                        if merge {
                            let msg_merge = _msg.msg.find_map(&mut vec!["msg".into()].iter());
        
                            if let Some(m) = msg_merge {
                                _msg = _msg.merge(usr.clone(), Unit::Map(m))?;
                            }
                        }
                        msg = _msg.merge(usr.clone(), u)?;
                    } else {
                        return Ok(None);
                    }

                    if net.len() - 1 == 1 || (i == net.len() - 2 && net.first().unwrap() != net.last().unwrap()) {
                        return Ok(Some(msg));
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn send<'b>(&'b mut self, serv: &str, msg: Msg) -> Result<Option<Msg>, KernErr> {
        let usr = self.users.iter().find(|usr| usr.name == msg.ath).ok_or(KernErr::UsrNotFound).cloned()?;
        usr.verify(&msg.msg, &msg.sign)?;

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
