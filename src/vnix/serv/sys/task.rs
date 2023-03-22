use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils::Maybe;
use crate::{thread, thread_await, read_async, as_map_find_async, maybe, as_map_find_as_async, as_async, maybe_ok, task_result};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::task::{ThreadAsync, TaskRun};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitModify, UnitAs, UnitNew};


pub const SERV_PATH: &'static str = "sys.task";
pub const SERV_HELP: &'static str = "Service for run task from message\nExample: (load @txt.hello)@io.store@sys.task";


fn stream(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Unit, Rc<String>), KernErr>> {
    thread!({
        let (msg, ath) = maybe!(read_async!(msg, ath, orig, kern));
        Ok(Some((msg, ath)))
    })
}

fn chain(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Unit, Rc<String>), KernErr>> {
    thread!({
        let (lst, mut ath) = maybe!(as_map_find_as_async!(msg, "task", as_list, ath, orig, kern));
        let mut _msg = if let Some((_msg, _ath)) = as_map_find_async!(msg, "msg", ath, orig, kern)? {
            ath = _ath;
            _msg
        } else {
            msg.clone()
        };

        for p in Rc::unwrap_or_clone(lst) {
            let (serv, _ath) = maybe!(as_async!(p, as_str, ath, orig, kern));
            let prev = _msg.clone();

            let run = TaskRun(_msg, Rc::unwrap_or_clone(serv));
            let id = kern.lock().reg_task(&_ath, "sys.task", run)?;

            let __msg = maybe_ok!(task_result!(id, kern)?);

            _msg = prev.merge_with(__msg.msg);
            ath = Rc::new(__msg.ath);
        }
        return Ok(Some((_msg, ath)))
    })
}

fn queue(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (lst, mut ath) = maybe!(as_map_find_as_async!(msg, "task.que", as_list, ath, orig, kern));

        for p in Rc::unwrap_or_clone(lst) {
            if let Some((_, _ath)) = read_async!(p, ath, orig, kern)? {
                ath = _ath;
            }
        }
        Ok(Some(ath))
    })
}

fn sim(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(), KernErr>> {
    thread!({
        let (lst, _) = maybe!(as_map_find_as_async!(msg, "task.sim", as_list, ath, orig, kern));

        for p in lst.iter() {
            if let Some((_msg, serv, _)) = p.clone().as_stream() {
                let run = TaskRun(_msg, serv);
                kern.lock().reg_task(&ath, "sys.task", run)?;
            }
        }
        Ok(None)
    })
}

fn stack(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (u, serv, _) = maybe_ok!(msg.as_map_find("task.stk").and_then(|u| u.as_stream()));
        let (lst, mut ath) = maybe!(as_async!(u, as_list, ath, orig, kern));

        for p in Rc::unwrap_or_clone(lst) {
            let (msg, _ath) = maybe!(read_async!(p, ath, orig, kern));
            ath = _ath;

            let run = TaskRun(msg, serv.clone());
            let id = kern.lock().reg_task(&ath, "sys.task", run)?;

            if let Some(msg) = task_result!(id, kern)? {
                ath = Rc::new(msg.ath);
            }
        }
        Ok(Some(ath))
    })
}

fn run(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Unit, Rc<String>), KernErr>> {
    thread!({
        let (msg, ath) = maybe!(read_async!(msg, ath, orig, kern));

        // chain
        if let Some((msg, ath)) = thread_await!(chain(ath.clone(), msg.clone(), msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)]
            );
            return Ok(Some((msg, ath)))
        }
    
        // sim
        thread_await!(sim(ath.clone(), msg.clone(), msg.clone(), kern))?;
    
        // queue
        if let Some(_ath) = thread_await!(queue(ath.clone(), msg.clone(), msg.clone(), kern))? {
            if _ath != ath {
                return Ok(Some((msg, ath)))
            }
        }

        // stack
        if let Some(_ath) = thread_await!(stack(ath.clone(), msg.clone(), msg.clone(), kern))? {
            if _ath != ath {
                return Ok(Some((msg, ath)))
            }
        }

        // stream
        if let Some((msg, ath)) = thread_await!(stream(ath.clone(), msg.clone(), msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), msg)]
            );
            return Ok(Some((msg, ath)))
        }

        Ok(None)
    })
}

pub fn task_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());

        // task
        if let Some((_msg, ath)) = thread_await!(run(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = msg.msg.merge_with(_msg);
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }
        Ok(Some(msg))
    })
}
