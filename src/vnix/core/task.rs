use core::ops::{Generator, GeneratorState};
use core::pin::Pin;

use alloc::boxed::Box;
use alloc::{string::String, vec::Vec};
use spin::Mutex;

use super::kern::{KernErr, Kern};
use super::msg::Msg;
use super::unit::Unit;


#[derive(Debug, Clone)]
pub enum TaskLoop {
    Sim(Vec<(Unit, String)>),
    Queue(Vec<(Unit, String)>),
    Chain {
        msg: Unit,
        chain: Vec<String>,
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub usr: String,
    pub name: String,
    pub id: usize,
    pub task: TaskLoop,
}

pub struct TaskRunAsync<'a>(pub Box<dyn Generator<Yield = (), Return = Result<Option<Msg>, KernErr>> + 'a>);

impl Task {
    pub fn new(usr: String, name: String, id: usize, task: TaskLoop) -> Self {
        Task{usr, name, id, task}
    }

    pub fn run<'a>(mut self, kern: &'a Mutex<Kern>) -> TaskRunAsync<'a> {
        TaskRunAsync(Box::new(
            move || {
                match self.task {
                    TaskLoop::Sim(sim) => {
                        let mut sim = sim.into_iter().map(|(u, serv)| {
                            let msg = kern.lock().msg(&self.usr, u).unwrap();
                            (Box::into_pin(Kern::send(kern, serv, msg).unwrap().unwrap().0), false)
                        }).collect::<Vec<_>>();

                        loop {
                            for (gen, done) in &mut sim {
                                if *done {
                                    continue;
                                }

                                if let GeneratorState::Complete(..) = Pin::new(gen).resume(()) {
                                    *done = true;
                                }
                            }

                            if sim.iter().all(|(_, done)| *done) {
                                return Ok(None)
                            }

                            yield;
                        }
                    },
                    TaskLoop::Queue(queue) => {
                        for (u, serv) in queue {
                            let msg = kern.lock().msg(&self.usr, u)?;

                            if let Some(gen) = Kern::send(kern, serv, msg)? {
                                let mut gen = Box::into_pin(gen.0);
    
                                loop {
                                    if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
                                        if let Some(msg) = res? {
                                            self.usr = msg.ath;
                                        }
                                        break;
                                    }
                                    yield;
                                }
                            }
                        }

                        Ok(None)
                    },
                    TaskLoop::Chain{mut msg, chain} => {
                        for serv in chain {
                            let mut _msg = kern.lock().msg(&self.usr, msg.clone())?;

                            if let Some(gen) = Kern::send(kern, serv, _msg)? {
                                let mut gen = Box::into_pin(gen.0);

                                loop {
                                    if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
                                        if let Some(_msg) = res? {
                                            self.usr = _msg.ath;
                                            msg = msg.merge(_msg.msg);
                                        } else {
                                            return Ok(None)
                                        }
                                        break;
                                    }
                                    yield;
                                }
                            }
                        }
                        kern.lock().msg(&self.usr, msg).map(|msg| Some(msg))
                    }
                }
            }
        ))
    }
}
