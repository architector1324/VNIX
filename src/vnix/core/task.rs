use core::ops::{Generator, GeneratorState};
use core::pin::Pin;

use alloc::boxed::Box;
use alloc::string::String;
use spin::Mutex;

use crate::vnix::utils::Maybe;

use super::msg::Msg;
use super::unit::Unit;
use super::kern::{KernErr, Kern};


#[derive(Debug, Clone)]
pub struct TaskRun(pub Unit, pub String);

#[derive(Debug, Clone)]
pub struct Task {
    pub usr: String,
    pub name: String,
    pub id: usize,
    pub parent_id: usize,
    pub run: TaskRun
}

#[derive(Debug, Clone)]
pub enum TaskSig {
    Kill
}

pub type TaskRunAsync<'a> = impl Generator<Yield = (), Return = Maybe<Msg, KernErr>> + 'a;
pub type ThreadAsync<'a, T> = Box<dyn Generator<Yield = (), Return = T> + 'a>;

#[macro_export]
macro_rules! thread {
    ($s:tt) => {
       {
            let hlr = move || $s;
            Box::new(hlr)
       } 
    };
}

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

#[macro_export]
macro_rules! task_result {
    ($id:expr, $kern:expr) => {
        {
            let res = loop {
                if let Some(res) = $kern.lock().get_task_result($id) {
                    break res;
                }
                yield;
            };
            res
        }
    };
}

impl Task {
    pub fn new(usr: String, name: String, id: usize, parent_id: usize, run: TaskRun) -> Self {
        Task{usr, name, id, parent_id, run}
    }

    pub fn run<'a>(self, kern: &'a Mutex<Kern>) -> TaskRunAsync<'a> {
        move || {
            let msg = kern.lock().msg(&self.usr, self.run.0)?;

            if let Some(run) = Kern::send(kern, self.run.1, msg)? {
                return thread_await!(run)
            }
            Ok(None)
        }
    }
}
