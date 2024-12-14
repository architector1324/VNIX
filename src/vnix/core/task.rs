use core::future::Future;
use core::pin::Pin;

use alloc::boxed::Box;
use alloc::string::String;
use spin::Mutex;

use crate::vnix::utils::Maybe;

use super::msg::Msg;
use super::unit::Unit;
use super::kern::{KernErr, Kern};

pub type ThreadAsync<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
pub type TaskRunAsync<'a> = ThreadAsync<'a, Maybe<Msg, KernErr>>;

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

impl Task {
    pub fn new(usr: String, name: String, id: usize, parent_id: usize, run: TaskRun) -> Self {
        Task{usr, name, id, parent_id, run}
    }

    pub fn run(self, kern: &Mutex<Kern>) -> TaskRunAsync {
        let f = async move || {
            let msg = kern.lock().msg(&self.usr, self.run.0)?;
            Kern::send(kern, self.run.1, msg).await
        };
        Box::pin(f())
    }
}
