use alloc::vec::Vec;

use super::msg::Msg;
use super::serv::{Serv, ServHlr, ServHelpTopic};
use super::serv::ServErr;
use super::unit::{Unit, UnitParseErr, SchemaMapEntry, SchemaSeq, SchemaUnit, SchemaStr, Schema, SchemaBool, SchemaMap, SchemaOr, Or};

use super::user::Usr;

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
    ServAlreadyReg,
    CannotCreateServInstance,
    DbLoadFault,
    DbSaveFault,
    HelpTopicNotFound,
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
    users: Vec<Usr>,
    services: Vec<Serv>
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
            services: Vec::new()
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

    pub fn reg_serv(&mut self, serv: Serv) -> Result<(), KernErr> {
        if self.services.iter().find(|s| s.name == serv.name).is_some() {
            return Err(KernErr::ServAlreadyReg);
        }

        self.services.push(serv);
        Ok(())
    }

    fn get_serv(&self, name: &str) -> Result<Serv, KernErr> {
        self.services.iter().find(|s| s.name == name).ok_or(KernErr::ServNotFound).cloned()
    }

    pub fn msg(&self, ath: &str, u: Unit) -> Result<Msg, KernErr> {
        let usr = self.get_usr(ath)?;
        Msg::new(usr, u)
    }

    fn msg_hlr(&self, msg: Msg, usr: Usr) -> Result<Option<Msg>, KernErr> {
        let schm = SchemaMap(
            SchemaMapEntry(Unit::Str("mrg".into()), SchemaUnit),
            SchemaMapEntry(Unit::Str("abt".into()), SchemaBool)
        );

        let u = msg.msg.clone();

        if let Some((msg, b)) = schm.find(&u) {
            if let Some(msg) = msg {
                return Ok(Some(self.msg(&usr.name, u.merge(msg))?));
            }

            if let Some(b) = b {
                if b {
                    return Ok(None)
                }
            }
        };

        Ok(Some(msg))
    }

    pub fn task(&mut self, msg: Msg) -> Result<Option<Msg>, KernErr> {
        let schm = SchemaMapEntry(
            Unit::Str("task".into()),
            SchemaOr(
                SchemaStr,
                SchemaSeq(SchemaUnit)
            )
        );

        if let Some(or) = schm.find(&msg.msg) {
            match or {
                Or::First(serv) => return self.send(serv.as_str(), msg),
                Or::Second(lst) => {
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
            }
        }

        Ok(None)
    }

    pub fn send<'b>(&'b mut self, serv: &str, mut msg: Msg) -> Result<Option<Msg>, KernErr> {
        // verify msg 
        let usr = self.get_usr(&msg.ath)?;
        usr.verify(&msg.msg, &msg.sign, &msg.hash)?;

        // prepare msg
        if let Some(_msg) = self.msg_hlr(msg, usr)? {
            msg = _msg;
        } else {
            return Ok(None);
        }
        
        let mut serv = self.get_serv(serv)?;
        let inst = serv.inst(&msg.msg).map_or(Err(KernErr::CannotCreateServInstance), |i| Ok(i))?;

        // check help
        if let Some(topic) = msg.msg.as_map_find("help").map(|u| u.as_str()).flatten() {
            match topic.as_str() {
                "info" => return inst.help(&msg.ath, ServHelpTopic::Info, self).map(|m| Some(m)),
                _ => return Err(KernErr::HelpTopicNotFound)
            }
        }
        
        // send
        inst.handle(msg, &mut serv, self)
    }
}
