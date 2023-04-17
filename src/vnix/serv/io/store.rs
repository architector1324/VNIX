use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::driver::MemSizeUnits;

use crate::vnix::utils::Maybe;
use crate::{thread, thread_await, read_async, as_map_find_async, as_async, maybe, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitParse, UnitModify, UnitReadAsyncI, UnitTypeReadAsync, UnitReadAsync};


pub const SERV_PATH: &'static str = "io.store";
const SERV_HELP: &'static str = "{
    name:io.store
    info:`Service for managing units disk storage`
    tut:[
        {
            info:`Load unit from storage`
            com:(load @txt.hello)@io.store
            res:`Hello, vnix!`
        }
        {
            info:`Load whole storage as unit`
            com:load@io.store
            res:`are u serious? :)`
        }
        {
            info:`Save text to storage`
            com:{save:abc out:@txt.test}@io.store
        }
        {
            info:`Get unit size in kb. from storage`
            com:(get.size.kb @img.vnix.logo)@io.store
            res:6
        }
    ]
    man:{
        load:{
            info:`Load unit from storage`
            schm:[
                load
                (load @path)
            ]
            tut:[@tut.0 @tut.1]
        }
        save:{
            info:`Save unit to storage`
            schm:[
                (save (unit @path))
                {save:unit out:@path}
            ]
            tut:@tut.2
        }
        get.size:{
            info:`Get unit size in bytes from storage`
            units:[kb mb gb]
            schm:[
                `get.size.<units>`
                (`get.size.<units>` @path)
            ]
            tut:@tut.3
        }
    }
}";

fn get_size(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<usize> {
    thread!({
        let (s, u, ath) = if let Some(s) = msg.clone().as_str() {
            // database
            (s, kern.lock().ram_store.data.clone(), ath)
        } else if let Some((u, path)) = msg.as_pair().into_iter().find_map(|(u0, u1)| Some((u0, u1.as_path()?))) {
            let (s, ath) = maybe!(as_async!(u, as_str, ath, orig, kern));
            // unit
            (s, kern.lock().ram_store.load(Unit::path_share(path)).ok_or(KernErr::DbLoadFault)?, ath)
        } else {
            return Ok(None);
        };

        let units = match s.as_str() {
            "get.size" => MemSizeUnits::Bytes,
            "get.size.kb" => MemSizeUnits::Kilo,
            "get.size.mb" => MemSizeUnits::Mega,
            "get.size.gb" => MemSizeUnits::Giga,
            _ => return Ok(None)
        };

        let size = u.size(units);
        Ok(Some((size, ath)))
    })
}

fn load(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitReadAsync {
    thread!({
        // load
        if let Some(s) = msg.clone().as_str() {
            if s.as_str() != "load" {
                return Ok(None)
            }

            let u = kern.lock().ram_store.data.clone();
            return Ok(Some((u, ath)))
        }

        // (load <path>)
        let (u, path) = maybe_ok!(msg.as_pair().into_iter().find_map(|(u0, u1)| Some((u0, u1.as_path()?))));
        let (s, ath) = maybe!(as_async!(u, as_str, ath, orig, kern));

        if s.as_str() == "load" {
            let u = kern.lock().ram_store.load(Unit::path_share(path)).ok_or(KernErr::DbLoadFault)?;
            return Ok(Some((u, ath)))
        }
        Ok(None)
    })
}

fn save(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<Rc<String>, KernErr>> {
    thread!({
        let (u, ath) = maybe!(as_map_find_async!(msg, "save", ath, orig, kern));
        let path = maybe_ok!(msg.as_map_find("out").and_then(|u| u.as_path()));

        kern.lock().ram_store.save(Unit::path_share(path), u);
        Ok(Some(ath))
    })
}

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let s = maybe_ok!(msg.msg.clone().as_str());
        let help = Unit::parse(SERV_HELP.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
        yield;

        let res = match s.as_str() {
            "help" => help,
            "help.name" => maybe_ok!(help.find(["name"].into_iter())),
            "help.info" => maybe_ok!(help.find(["info"].into_iter())),
            "help.tut" => maybe_ok!(help.find(["tut"].into_iter())),
            "help.man" => maybe_ok!(help.find(["man"].into_iter())),
            _ => return Ok(None)
        };

        let _msg = Unit::map(&[
            (Unit::str("msg"), res)
        ]);
        kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
    })
}

pub fn store_hlr(mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // get size
        if let Some((size, ath)) = thread_await!(get_size(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(size as u32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // load
        if let Some((u, ath)) = thread_await!(load(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), u)]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // save
        if let Some(_ath) = thread_await!(save(ath.clone(), _msg.clone(), _msg.clone(), kern))? {
            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), _msg)?;
            }
        }

        Ok(Some(msg))
    })
}
