use core::pin::Pin;
use core::fmt::{Display, Write};
use core::ops::{Generator, GeneratorState};

use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use super::msg::Msg;
use super::task::{Task, TaskRun, TaskSig};
use super::unit::{Unit, UnitParseErr, UnitAs, UnitNew};
use super::serv::{Serv, ServErr, ServHlrAsync};

use super::user::Usr;

use crate::thread;
use crate::driver::{CLIErr, CLI, Disp, Time, Rnd, Mem, DrvErr};

use crate::vnix::serv::io::term::base;
use crate::vnix::utils::{RamStore, Maybe};

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
    TaskNotFound,
    DbLoadFault,
    DbSaveFault,
    HelpTopicNotFound,
    ParseErr(UnitParseErr),
    DrvErr(DrvErr),
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
    pub term: Rc<Mutex<base::Term>>,
    pub ram_store: RamStore,

    // vnix
    users: Vec<Usr>,
    services: Vec<Serv>,

    last_task_id: usize,
    curr_task_id: usize,
    tasks_queue: Vec<Task>,
    tasks_running: Vec<Task>,
    tasks_signals: Vec<(usize, TaskSig)>,
    task_result: Vec<(usize, Maybe<Msg, KernErr>)>
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
    pub fn new(drv: KernDrv, term: Rc<Mutex<base::Term>>) -> Self {
        let kern = Kern {
            drv,
            ram_store: RamStore::default(),
            term,
            users: Vec::new(),
            services: Vec::new(),
            last_task_id: 0,
            curr_task_id: 0,
            tasks_queue: Vec::new(),
            tasks_running: Vec::new(),
            tasks_signals: Vec::new(),
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
        if self.services.iter().find(|s| s.info.name == serv.info.name).is_some() {
            return Err(KernErr::ServAlreadyReg);
        }

        self.services.push(serv);
        Ok(())
    }

    pub fn reg_task(&mut self, usr: &str, name: &str, run: TaskRun) -> Result<usize, KernErr> {
        self.tasks_queue.push(Task::new(usr.into(), name.into(), self.last_task_id, self.curr_task_id, run));
        self.last_task_id += 1;
        Ok(self.last_task_id - 1)
    }

    pub fn task_sig(&mut self, id: usize, sig: TaskSig) -> Result<(), KernErr> {
        self.tasks_signals.push((id, sig));
        Ok(())
    }

    fn get_serv(&self, name: &str) -> Result<&Serv, KernErr> {
        self.services.iter().find(|s| s.info.name == name).ok_or(KernErr::ServNotFound)
    }

    pub fn get_tasks_running(&self) -> Vec<Task> {
        self.tasks_running.clone()
    }

    pub fn get_task_running(&self) -> usize {
        self.curr_task_id
    }

    pub fn get_task_result(&mut self, id: usize) -> Option<Maybe<Msg, KernErr>> {
        self.task_result.drain_filter(|(i, _)| *i == id).next().map(|(_, msg)| msg)
    }

    pub fn msg(&self, ath: &str, u: Unit) -> Result<Msg, KernErr> {
        let usr = self.get_usr(ath)?;
        Msg::new(usr, u)
    }

    fn help_serv(&self, ath: &str) -> Result<Msg, KernErr> {
        let serv = self.services.iter().map(|serv| Unit::str(&serv.info.name)).collect::<Vec<_>>();
        let u = Unit::map(&[(
            Unit::str("msg"),
            Unit::list(&serv)
        )]);

        self.msg(ath, u)
    }

    pub fn send<'a>(mtx: &'a Mutex<Self>, serv: String, msg: Msg) -> Maybe<ServHlrAsync<'a>, KernErr> {
        // verify msg
        let usr = mtx.lock().get_usr(&msg.ath)?;
        usr.verify(msg.msg.clone(), &msg.sign, &msg.hash)?;

        // prepare msg
        let tmp = mtx.lock();
        let serv = tmp.get_serv(serv.as_str())?;

        let help_msg = Unit::map(&[
            (Unit::str("msg"), Unit::str(&serv.help))
        ]);
        let help_msg = tmp.msg(&msg.ath, help_msg)?;

        // check help
        let topic = if let Some(topic) = msg.msg.clone().as_map_find("help").map(|u| u.as_str()).flatten() {
            Some(topic)
        } else if let Some(topic) = msg.msg.clone().as_str() {
            Some(topic)
        } else {
            None
        };

        if let Some(topic) = topic {
            match topic.as_str() {
                "info" | "help" => return Ok(Some(thread!({
                    yield;
                    Ok(Some(help_msg))
                }))),
                "serv" => return Ok(Some(thread!({
                    let out = mtx.lock().help_serv(&msg.ath).map(|m| Some(m));
                    yield;
                    out
                }))),
                _ => ()
            }
        }

        // send
        let inst = (serv.hlr)(msg, serv.info.clone(), mtx);
        Ok(Some(inst))
    }

    pub fn run<'a>(self) -> Result<(), KernErr> {
        let kern_mtx = Mutex::new(self);

        loop {
            let mut runs = kern_mtx.lock().tasks_queue.clone().into_iter().map(|t| {
                let task = t.clone();
                let run = t.run(&kern_mtx);

                (task, (run, false))
            }).collect::<Vec<_>>();

            kern_mtx.lock().tasks_queue = Vec::new();

            // run tasks
            for (task, _) in runs.iter() {
                kern_mtx.lock().tasks_running.push(task.clone());
            }

            loop {
                for (task, (run, done)) in &mut runs {
                    // check signals
                    if let Some(sig) = kern_mtx.lock().tasks_signals.iter().find(|(id, _)| *id == task.id).map(|(_, sig)| sig.clone()) {
                        match sig {
                            TaskSig::Kill => {
                                // writeln!(kern_mtx.lock(), "DEBG vnix:kern: killed task `{}#{}`", task.name, task.id).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
                                *done = true
                            }
                        }
                    }

                    if *done {
                        kern_mtx.lock().tasks_running.drain_filter(|t| t.id == task.id).next();
                        continue;
                    }

                    // run task
                    kern_mtx.lock().curr_task_id = task.id;

                    if let GeneratorState::Complete(res) = Pin::new(run).resume(()) {
                        match &res {
                            Ok(..) => (), // writeln!(kern_mtx.lock(), "DEBG vnix:kern: done task `{}#{}`", task.name, task.id).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?,
                            Err(e) => {
                                writeln!(kern_mtx.lock(), "ERR vnix:{}#{}: {:?}", task.name, task.id, e).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
                                // writeln!(kern_mtx.lock(), "DEBG vnix:kern: killed task `{}#{}`", task.name, task.id).map_err(|_| KernErr::CLIErr(CLIErr::Write))?
                            }
                        };

                        kern_mtx.lock().task_result.push((task.id, res));
                        *done = true;
                    }
                }

                // run new tasks
                if !kern_mtx.lock().tasks_queue.is_empty() {
                    let mut new_runs = kern_mtx.lock().tasks_queue.clone().into_iter().map(|t| {
                        let task = t.clone();
                        let run = t.run(&kern_mtx);

                        (task, (run, false))
                    }).collect::<Vec<_>>();

                    kern_mtx.lock().tasks_queue = Vec::new();

                    for (task, _) in new_runs.iter() {
                        kern_mtx.lock().tasks_running.push(task.clone());
                        // writeln!(kern_mtx.lock(), "DEBG vnix:kern: run task `{}#{}`", task.name, task.id).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
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

impl Write for Kern {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let term = self.term.clone();
        term.lock().print(s, self).ok();
        Ok(())
    }
}