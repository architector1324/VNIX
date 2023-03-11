use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::vec;
use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;
use crate::vnix::core::task::{ThreadAsync, TaskRun};
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "sys.task";
pub const SERV_HELP: &'static str = "Service for run task from message\nExample: (load @txt.hello)@io.store@sys.task";


fn chain(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(Unit, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((lst, mut ath)) = as_map_find_async!(msg, "task", ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_vec()?, ath))) {
            let mut _msg = Rc::unwrap_or_clone(msg.clone());

            for p in lst {
                if let Some((serv, _ath)) = read_async!(Rc::new(p), ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_str()?, ath))) {
                    let prev = _msg.clone();

                    let run = TaskRun(_msg, serv);
                    let id = kern.lock().reg_task(&_ath, "sys,task", run)?;

                    loop {
                        if let Some(res) = kern.lock().get_task_result(id) {
                            if let Some(__msg) = res? {
                                _msg = prev.merge(__msg.msg);
                                ath = Rc::new(__msg.ath);
                                break;
                            }
                            return Ok(None)
                        }
                        yield;
                    }
                } else {
                    return Ok(None)
                }
            }

            return Ok(Some((_msg, ath)))
        }
        Ok(None)
    })
}

fn queue(mut ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Rc<String>, KernErr>> {
    thread!({
        if let Some((lst, _ath)) = as_map_find_async!(msg, "task.que", ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_vec()?, ath))) {
            ath = _ath;

            for p in lst {
                if let Some((_, _ath)) = read_async!(Rc::new(p), ath, orig, kern)? {
                    ath = _ath;
                }
            }
        }
        Ok(ath)
    })
}

fn sim(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<(), KernErr>> {
    thread!({
        if let Some((lst, _)) = as_map_find_async!(msg, "task.sim", ath, orig, kern)?.and_then(|(u, ath)| Some((u.as_vec()?, ath))) {
            for p in lst {
                if let Some((_msg, (serv, _))) = p.as_stream() {
                    let run = TaskRun(_msg, serv);
                    kern.lock().reg_task(&ath, "sys.task", run)?;
                }
            }
        }
        Ok(())
    })
}

pub fn task_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());
        let mut ath = Rc::new(msg.ath.clone());

        if let Some((u, _ath)) = read_async!(u.clone(), ath, u, kern)? {
            ath = _ath;

            // chain
            if let Some((u, ath)) = thread_await!(chain(ath.clone(), u.clone(), u.clone(), kern))? {
                let m = Unit::Map(vec![
                    (Unit::Str("msg".into()), u)]
                );

                let _msg = msg.msg.merge(m);
                return kern.lock().msg(&ath, _msg).map(|msg| Some(msg))
            }
    
            // sim
            thread_await!(sim(ath.clone(), u.clone(), u.clone(), kern))?;

            // queue
            let _ath = thread_await!(queue(ath.clone(), u.clone(), u.clone(), kern))?;

            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), msg.msg)?;
            }
        }

        Ok(Some(msg))
    })
}
