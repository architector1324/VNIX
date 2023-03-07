use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::vec;
use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::MemSizeUnits;

use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, SchemaPair, SchemaUnit, SchemaRef, Schema};
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "io.store";
pub const SERV_HELP: &'static str = "Disk units storage service\nExample: {save:`Some beautiful text` out:@txt.doc}@io.store # save text to `txt.doc` path\n(load @txt.doc)@io.store";


fn get_size(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<usize>, KernErr>> {
    thread!({
        if let Some(res) = read_async!(msg, ath, orig, kern)? {
            // database size
            if let Some(s) = res.as_str() {
                let units = match s.as_str() {
                    "get.size" => MemSizeUnits::Bytes,
                    "get.size.kb" => MemSizeUnits::Kilo,
                    "get.size.mb" => MemSizeUnits::Mega,
                    "get.size.gb" => MemSizeUnits::Giga,
                    _ => return Ok(None)
                };
    
                let size = kern.lock().ram_store.data.size(units);
                return Ok(Some(size))
            }

            // unit size
            let schm = SchemaPair(SchemaUnit, SchemaRef);

            if let Some((s, path)) = schm.find(&orig, &msg) {
                if let Some(s) = read_async!(Rc::new(s), ath, orig, kern)?.and_then(|s| s.as_str()) {
                    let units = match s.as_str() {
                        "get.size" => MemSizeUnits::Bytes,
                        "get.size.kb" => MemSizeUnits::Kilo,
                        "get.size.mb" => MemSizeUnits::Mega,
                        "get.size.gb" => MemSizeUnits::Giga,
                        _ => return Ok(None)
                    };

                    let size = kern.lock().ram_store.load(Unit::Ref(path)).ok_or(KernErr::DbLoadFault)?.size(units);
                    return Ok(Some(size))
                }
            }
        }

        Ok(None)
    })
}

fn load(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<Unit>, KernErr>> {
    thread!({
        let schm = SchemaPair(SchemaUnit, SchemaRef);

        if let Some((s, path)) = schm.find(&orig, &msg) {
            if let Some(s) = read_async!(Rc::new(s), ath, orig, kern)?.and_then(|s| s.as_str()) {
                if s == "load" {
                    let u = kern.lock().ram_store.load(Unit::Ref(path)).ok_or(KernErr::DbLoadFault)?;
                    return Ok(Some(u))
                }
            }
        }
        Ok(None)
    })
}

fn save(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<(), KernErr>> {
    thread!({
        if let Some(u) = as_map_find_async!(msg, "save", ath, orig, kern)? {
            if let Some(path) = msg.as_map_find("out").and_then(|u| u.as_ref()) {
                kern.lock().ram_store.save(Unit::Ref(path), u);
            }
        }
        Ok(())
    })
}

pub fn store_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());
        let ath = Rc::new(msg.ath.clone());

        // get size
        if let Some(size) = thread_await!(get_size(ath.clone(), u.clone(), u.clone(), kern))? {
            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), Unit::Int(size as i32))]
            );

            let _msg = msg.msg.merge(m);
            return kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
        }

        // load
        if let Some(u) = thread_await!(load(ath.clone(), u.clone(), u.clone(), kern))? {
            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), u)]
            );

            let _msg = msg.msg.merge(m);
            return kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
        }

        // save
        thread_await!(save(ath, u.clone(), u, kern))?;

        Ok(Some(msg))
    })
}