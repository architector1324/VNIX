use core::future::Future;
use core::pin::Pin;
use futures::task::{Context, Poll};

use alloc::boxed::Box;
use alloc::string::String;
use spin::Mutex;

use crate::vnix::utils::Maybe;

use super::msg::Msg;
use super::unit::Unit;
use super::kern::{KernErr, Kern};

pub type ThreadAsync<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
pub type TaskRunAsync<'a> = ThreadAsync<'a, Maybe<Msg, KernErr>>;


pub struct Yield {
    done: bool
}

impl Yield {
    pub fn now() -> Self {
        Self {done: false}
    }
}

#[macro_export]
macro_rules! thread {
    ($f:expr) => {
        {
            let tmp = async move || $f;
            Box::pin(tmp())
        }
    }
}

#[macro_export]
macro_rules! task_result {
    ($id:expr, $kern:expr) => {
        {
            use crate::vnix::core::task;
            let res = loop {
                if let Some(res) = $kern.lock().get_task_result($id) {
                    break res;
                }
                task::Yield::now().await;
            };
            res
        }
    };
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done {
            Poll::Ready(())
        } else {
            self.done = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

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
        thread!({
            let msg = kern.lock().msg(&self.usr, self.run.0)?;
            Kern::send(kern, self.run.1, msg).await
        })
    }
}
