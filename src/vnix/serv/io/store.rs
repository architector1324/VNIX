use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use spin::Mutex;
use async_trait::async_trait;

use crate::vnix::utils::Maybe;
use crate::{read_async, as_map_find_async, as_async, maybe, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::driver::MemSizeUnits;

use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};
use crate::vnix::core::unit::{Unit, UnitNew, UnitAs, UnitTypeAsyncResult, UnitReadAsyncI, UnitAsyncResult};


pub const SERV_PATH: &'static str = "io.store";
pub const SERV_HELP: &'static str = "{
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

pub struct StoreHlr;

impl StoreHlr {
    async fn get_size(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeAsyncResult<usize> {
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
    }
    
    async fn load(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitAsyncResult {
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
    }
    
    async fn save(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> Maybe<Rc<String>, KernErr> {
        let (u, ath) = maybe!(as_map_find_async!(msg, "save", ath, orig, kern));
        let path = maybe_ok!(msg.as_map_find("out").and_then(|u| u.as_path()));

        kern.lock().ram_store.save(Unit::path_share(path), u);
        Ok(Some(ath))
    }
}


#[async_trait(?Send)]
impl ServHlr for StoreHlr {
    async fn hlr(&self, mut msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
        let ath = Rc::new(msg.ath.clone());
        let (_msg, ath) = maybe!(read_async!(msg.msg.clone(), ath.clone(), msg.msg.clone(), kern));

        // get size
        if let Some((size, ath)) = Self::get_size(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::uint(size as u32))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // load
        if let Some((u, ath)) = Self::load(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            let msg = Unit::map(&[
                (Unit::str("msg"), u)]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        // save
        if let Some(_ath) = Self::save(ath.clone(), _msg.clone(), _msg.clone(), kern).await? {
            if ath != _ath {
                msg = kern.lock().msg(&_ath.clone(), _msg)?;
            }
        }

        Ok(Some(msg))
    }
}
