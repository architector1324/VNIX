use core::fmt::Display;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use super::msg::Msg;
use super::serv::{Serv, ServHlr, ServHelpTopic};
use super::serv::ServErr;
use super::unit::{Unit, UnitParseErr, SchemaMapEntry, SchemaSeq, SchemaUnit, SchemaStr, Schema, SchemaBool, SchemaMap, SchemaOr, Or};

use super::user::Usr;

use crate::driver::{CLIErr, DispErr, TimeErr, RndErr, CLI, Disp, Time, Rnd, Mem, MemErr};
use crate::vnix::serv::io::term::TermBase;
use crate::vnix::utils::RamStore;


#[derive(Debug, PartialEq, Clone)]
pub enum Addr {
    Local,
    Remote([u16; 8])
}

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
    MemErr(MemErr),
    ServErr(ServErr)
}

pub struct KernDrv {
    pub cli: Box<dyn CLI>,
    pub disp: Box<dyn Disp>,
    pub time: Box<dyn Time>,
    pub rnd: Box<dyn Rnd>,
    pub mem: Box<dyn Mem>,
}

pub struct Kern {
    pub drv: KernDrv,
    pub term: TermBase,
    pub ram_store: RamStore,

    // vnix
    users: Vec<Usr>,
    services: Vec<Serv>
}

impl KernDrv {
    pub fn new(cli: Box<dyn CLI>, disp: Box<dyn Disp>, time: Box<dyn Time>, rnd: Box<dyn Rnd>, mem: Box<dyn Mem>) -> Self {
        KernDrv {
            cli,
            disp,
            time,
            rnd,
            mem
        }
    }
}

impl Kern {
    pub fn new(drv: KernDrv) -> Self {
        let kern = Kern {
            drv,
            ram_store: RamStore::default(),
            term: TermBase::default(),
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

        if let Some((msg, b)) = schm.find_loc(&u) {
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

        if let Some(or) = schm.find_loc(&msg.msg) {
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

    fn help_serv(&self, ath: &str) -> Result<Msg, KernErr> {
        let serv = self.services.iter().cloned().map(|serv| Unit::Str(serv.name)).collect();
        let u = Unit::Map(vec![(
            Unit::Str("msg".into()),
            Unit::Lst(serv)
        )]);

        self.msg(ath, u)
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
        let mut inst = serv.inst(&msg.msg).ok_or(KernErr::CannotCreateServInstance)?;

        // check help
        let topic = if let Some(topic) = msg.msg.as_map_find("help").map(|u| u.as_str()).flatten() {
            Some(topic)
        } else if let Some(topic) = msg.msg.as_str() {
            Some(topic)
        } else {
            None
        };

        if let Some(topic) = topic {
            match topic.as_str() {
                "info" => return inst.help(&msg.ath, ServHelpTopic::Info, self).map(|m| Some(m)),
                "serv" => return self.help_serv(&msg.ath).map(|m| Some(m)),
                _ => ()
            }
        }

        // send
        inst.handle(msg, &mut serv, self)
    }
}

impl Display for Addr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Addr::Local => write!(f, "loc"),
            Addr::Remote(addr) => write!(f,
                "{:#04x}:{:#04x}:{:#04x}:{:#04x}:{:#04x}:{:#04x}:{:#04x}:{:#04x}",
                addr[0], addr[1], addr[2], addr[3],
                addr[4], addr[5], addr[6], addr[7]
            )
        }
    }
}
