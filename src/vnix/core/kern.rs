use alloc::vec;
use alloc::vec::Vec;

use super::msg::Msg;
use super::serv::Serv;
use super::serv::ServErr;
use super::unit::Unit;
use super::unit::UnitParseErr;

use super::user::Usr;

use crate::vnix::serv::{io, etc, gfx};

use crate::driver::{CLIErr, DispErr, TimeErr, CLI, Disp, Time};

#[derive(Debug)]
pub enum KernErr {
    MemoryOut,
    EncodeFault,
    UsrNotFound,
    ServNotFound,
    ParseErr(UnitParseErr),
    CLIErr(CLIErr),
    DispErr(DispErr),
    TimeErr(TimeErr),
    ServErr(ServErr)
}

pub struct Kern<'a> {
    // drivers
    pub cli: &'a mut dyn CLI,
    pub disp: &'a mut dyn Disp,
    pub time: &'a mut dyn Time,

    // vnix
    users: Vec<Usr>
}

impl<'a> Kern<'a> {
    pub fn new(cli: &'a mut dyn CLI, disp: &'a mut dyn Disp, time: &'a mut dyn Time) -> Self {
        let kern = Kern {
            cli,
            disp,
            time,
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
            let u = msg.msg.clone();

            if let Some(_msg) = self.send(net.first().unwrap().as_str(), msg)? {
                msg = _msg.merge(u)?;
            } else {
                return Ok(None);
            }

            loop {
                for (i, serv) in net.iter().skip(1).enumerate() {
                    let u = msg.msg.clone();
                    // let ath = msg.ath.clone();
    
                    if let Some(_msg) = self.send(serv.as_str(), msg)? {
                        msg = _msg.merge(u)?;
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

    pub fn send(&mut self, serv: &str, msg: Msg) -> Result<Option<Msg>, KernErr> {
        match serv {
            "io.term" => {
                let (inst, msg) = io::Term::inst(msg, self)?;
                inst.handle(msg, self)
            },
            "etc.chrono" => {
                let (inst, msg) = etc::chrono::Chrono::inst(msg, self)?;
                inst.handle(msg, self)
            },
            "etc.fsm" => {
                let (inst, msg) = etc::fsm::FSM::inst(msg, self)?;
                inst.handle(msg, self)
            },
            "gfx.2d" => {
                let (inst, msg) = gfx::GFX2D::inst(msg, self)?;
                inst.handle(msg, self)
            }
            _ => Err(KernErr::ServNotFound)
        }
    }
}
