use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::vec;
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::{ThreadAsync, TaskRun};
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::unit::{Unit, SchemaPair, SchemaUnit, Schema, SchemaSeq};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "sys.task";
pub const SERV_HELP: &'static str = "Service for run task from message\nExample: (load @txt.hello)@io.store@sys.task";


fn chain(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<Unit>, KernErr>> {
    thread!({
        if let Some(lst) = as_map_find_async!(msg, "task", ath, orig, kern)?.and_then(|u| u.as_vec()) {
            // let _msg = 
            for p in lst {

            }
        }
        Ok(None)
    })
}

fn queue(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<(), KernErr>> {
    thread!({
        if let Some(lst) = as_map_find_async!(msg, "task.que", ath, orig, kern)?.and_then(|u| u.as_vec()) {
            for p in lst {
                read_async!(Rc::new(p), ath, orig, kern)?;
            }
        }
        Ok(())
    })
}

fn sim(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<(), KernErr>> {
    thread!({
        if let Some(lst) = as_map_find_async!(msg, "task.sim", ath, orig, kern)?.and_then(|u| u.as_vec()) {
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

pub fn task_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());
        let ath = Rc::new(msg.ath.clone());

        if let Some(u) = read_async!(u.clone(), ath, u, kern)? {
            // chain
            if let Some(u) = thread_await!(chain(ath.clone(), u.clone(), u.clone(), kern))? {
                let m = Unit::Map(vec![
                    (Unit::Str("msg".into()), u)]
                );
    
                let _msg = msg.msg.merge(m);
                return kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
            }
    
            // sim
            thread_await!(sim(ath.clone(), u.clone(), u.clone(), kern))?;
    
            // queue
            thread_await!(queue(ath.clone(), u.clone(), u.clone(), kern))?;
        }

        Ok(Some(msg))
    })
}
