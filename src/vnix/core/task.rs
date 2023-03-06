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
    },
    ChainLoop {
        msg: Unit,
        chain: Vec<String>,
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub usr: String,
    pub name: String,
    pub id: usize,
    pub parent_id: usize,
    pub task: TaskLoop,
}

#[derive(Debug, Clone)]
pub enum TaskSig {
    Kill
}

pub type TaskRunAsync<'a> = impl Generator<Yield = (), Return = Result<Option<Msg>, KernErr>> + 'a;
pub type ThreadAsync<'a, T> = Box<dyn Generator<Yield = (), Return = T> + 'a>;

#[macro_export]
macro_rules! thread_await {
    ($t:expr) => {
        {
            let mut gen = Box::into_pin($t);
            let res = loop {
                if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
                    break res;
                }
                yield;
            };
            res
        }
    };
}

impl Task {
    pub fn new(usr: String, name: String, id: usize, parent_id: usize, task: TaskLoop) -> Self {
        Task{usr, name, id, parent_id, task}
    }

    pub fn run<'a>(mut self, kern: &'a Mutex<Kern>) -> TaskRunAsync<'a> {
        move || {
            match self.task {
                TaskLoop::Sim(sim) => {
                    let mut sim = sim.into_iter().map(|(u, serv)| {
                        let msg = kern.lock().msg(&self.usr, u).ok()?;
                        let gen = Box::into_pin(Kern::send(kern, serv, msg).ok()??);
                        Some((gen, false))
                    }).collect::<Option<Vec<_>>>().ok_or(KernErr::CannotCreateServInstance)?;

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
                            if let Some(msg) = thread_await!(gen)? {
                                self.usr = msg.ath;
                            }
                        }
                    }

                    Ok(None)
                },
                TaskLoop::Chain{mut msg, chain} => {
                    for serv in chain {
                        let mut _msg = kern.lock().msg(&self.usr, msg.clone())?;

                        if let Some(gen) = Kern::send(kern, serv, _msg)? {
                            if let Some(_msg) = thread_await!(gen)? {
                                self.usr = _msg.ath;
                                msg = msg.merge(_msg.msg);
                            } else {
                                return Ok(None)
                            }
                        }
                    }
                    kern.lock().msg(&self.usr, msg).map(|msg| Some(msg))
                },
                TaskLoop::ChainLoop{mut msg, chain} => {
                    loop {
                        for serv in chain.clone() {
                            let mut _msg = kern.lock().msg(&self.usr, msg.clone())?;

                            if let Some(gen) = Kern::send(kern, serv, _msg)? {
                                if let Some(_msg) = thread_await!(gen)? {
                                    self.usr = _msg.ath;
                                    msg = msg.merge(_msg.msg);
                                } else {
                                    return Ok(None)
                                }
                            }
                        }
                        yield;
                    }
                }
            }
        }
    }
}
