use alloc::vec;
use alloc::vec::Vec;

use super::msg::Msg;
use super::serv::{Serv, ServHlr};
use super::serv::ServErr;
use super::unit::{Unit, UnitParseErr, FromUnit};

use super::user::Usr;

use crate::vnix::serv::{io, etc, gfx, math, sys};

use crate::driver::{CLIErr, DispErr, TimeErr, RndErr, CLI, Disp, Time, Rnd};
use crate::vnix::utils::RamDB;

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
    HashVerifyFault,
    UsrNotFound,
    UsrNameAlreadyReg,
    UsrAlreadyReg,
    UsrRegWithAnotherName,
    ServNotFound,
    DbLoadFault,
    DbSaveFault,
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
    pub db_ram: RamDB,

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
            db_ram: RamDB::default(),
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

    fn msg_hlr(&self, msg: Msg, usr: Usr) -> Result<Option<Msg>, KernErr> {
        if let Some(_msg) = msg.msg.find_unit(&mut vec!["mrg".into()].iter()) {
            return Ok(Some(self.msg(&usr.name, msg.msg.merge(_msg))?));
        }

        if let Some(b) = msg.msg.find_bool(&mut vec!["abt".into()].iter()) {
            if b {
                return Ok(None)
            }
        }

        Ok(Some(msg))
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
                        msg = self.msg(&usr.name, u.merge(_msg.msg))?;
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

    pub fn send<'b>(&'b mut self, serv: &str, mut msg: Msg) -> Result<Option<Msg>, KernErr> {
        let usr = self.get_usr(&msg.ath)?;
        usr.verify(&msg.msg, &msg.sign, &msg.hash)?;

        if let Some(_msg) = self.msg_hlr(msg, usr)? {
            msg = _msg;
        } else {
            return Ok(None);
        }

        let mut _serv = Serv {
            name: serv.into(),
            kern: self,
        };

        match serv {
            "io.term" => {
                let inst = io::term::Term::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            "io.db" => {
                let inst = io::db::DB::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            "etc.chrono" => {
                let inst = etc::chrono::Chrono::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            "etc.fsm" => {
                let inst = etc::fsm::FSM::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            "gfx.2d" => {
                let inst = gfx::GFX2D::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            "math.int" => {
                let inst = math::Int::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            "sys.task" => {
                let inst = sys::task::Task::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            "sys.usr" => {
                let inst = sys::usr::User::from_unit(&msg.msg);
                inst.map_or(Ok(None), |inst| inst.handle(msg, &mut _serv))
            },
            _ => Err(KernErr::ServNotFound)
        }
    }
}
