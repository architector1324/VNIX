use core::pin::Pin;
use core::fmt::Display;
use core::ops::{Generator, GeneratorState};

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;

use super::msg::Msg;
use super::task::{Task, TaskLoop};
use super::unit::{Unit, UnitParseErr, SchemaMapEntry, SchemaUnit, Schema, SchemaBool, SchemaMap};
use super::serv::{Serv, ServHlr, ServErr, ServHelpTopic, ServHlrAsync};

use super::user::Usr;

use crate::driver::{CLIErr, DispErr, TimeErr, RndErr, CLI, Disp, Time, Rnd, Mem, MemErr};
use crate::vnix::serv::io::term::TermBase;
use crate::vnix::utils::RamStore;

use spin::Mutex;


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
    TaskAlreadyReg,
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
    services: Vec<Serv>,
    last_task_id: usize,
    tasks: Vec<Task>,
    task_result: Vec<(usize, Option<Msg>)>
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
            services: Vec::new(),
            last_task_id: 0,
            tasks: Vec::new(),
            task_result: Vec::new()
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

    pub fn reg_task(&mut self, usr: &str, name: &str, task: TaskLoop) -> Result<usize, KernErr> {
        self.tasks.push(Task::new(usr.into(), name.into(), self.last_task_id, task));
        self.last_task_id += 1;
        Ok(self.last_task_id - 1)
    }

    fn get_serv(&self, name: &str) -> Result<Serv, KernErr> {
        self.services.iter().find(|s| s.name == name).ok_or(KernErr::ServNotFound).cloned()
    }

    pub fn get_task_result(&mut self, id: usize) -> Option<Option<Msg>> {
        self.task_result.drain_filter(|(i, _)| *i == id).next().map(|(_, msg)| msg)
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

    fn help_serv(&self, ath: &str) -> Result<Msg, KernErr> {
        let serv = self.services.iter().cloned().map(|serv| Unit::Str(serv.name)).collect();
        let u = Unit::Map(vec![(
            Unit::Str("msg".into()),
            Unit::Lst(serv)
        )]);

        self.msg(ath, u)
    }

    pub fn send<'a>(mtx: &'a Mutex<Self>, serv: String, mut msg: Msg) -> Result<Option<ServHlrAsync<'a>>, KernErr> {
        // verify msg
        let usr = mtx.lock().get_usr(&msg.ath)?;
        usr.verify(&msg.msg, &msg.sign, &msg.hash)?;

        // prepare msg
        if let Some(_msg) = mtx.lock().msg_hlr(msg, usr)? {
            msg = _msg;
        } else {
            return Ok(None);
        }

        let serv = mtx.lock().get_serv(serv.as_str())?;
        let inst = serv.inst(&msg.msg).ok_or(KernErr::CannotCreateServInstance)?;

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
                "info" => return Ok(Some(inst.help(msg.ath, ServHelpTopic::Info, mtx))),
                "serv" => return Ok(Some(ServHlrAsync(Box::new(move || {
                    let out = mtx.lock().help_serv(&msg.ath).map(|m| Some(m));
                    yield;
                    out
                })))),
                _ => ()
            }
        }

        // send
        Ok(Some(inst.handle(msg, serv, mtx)))
    }

    pub fn run<'a>(self) -> Result<(), KernErr> {
        let kern_mtx = Mutex::new(self);

        loop {
            let mut runs = kern_mtx.lock().tasks.clone().into_iter().map(|t| {
                let task = t.clone();
                let run = Box::into_pin(t.run(&kern_mtx).0);

                (task, (run, false))
            }).collect::<Vec<_>>();

            kern_mtx.lock().tasks = Vec::new();

            for (task, _) in runs.iter() {
                writeln!(kern_mtx.lock().drv.cli, "INFO vnix:kern: run task `{}#{}`", task.name, task.id).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
            }

            loop {
                for (task, (run, done)) in &mut runs {
                    if *done {
                        continue;
                    }

                    if let GeneratorState::Complete(res) = Pin::new(run).resume(()) {
                        kern_mtx.lock().task_result.push((task.id, res?));
                        *done = true;
                    }
                }

                // run new tasks
                if !kern_mtx.lock().tasks.is_empty() {
                    let mut new_runs = kern_mtx.lock().tasks.clone().into_iter().map(|t| {
                        let task = t.clone();
                        let run = Box::into_pin(t.run(&kern_mtx).0);
        
                        (task, (run, false))
                    }).collect::<Vec<_>>();

                    kern_mtx.lock().tasks = Vec::new();
        
                    for (task, _) in new_runs.iter() {
                        writeln!(kern_mtx.lock().drv.cli, "INFO vnix:kern: run task `{}#{}`", task.name, task.id).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    }

                    runs.append(&mut new_runs);
                }

                // done
                if runs.iter().all(|(_, (_, done))| *done) {
                    break;
                }
            }
        }
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
