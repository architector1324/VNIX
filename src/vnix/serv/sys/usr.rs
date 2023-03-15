use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::{DrvErr, CLIErr};

use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::user::Usr;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitNew, UnitAs, UnitParse};


pub const SERV_PATH: &'static str = "sys.usr";
pub const SERV_HELP: &'static str = "Users management service\nExample: {ath:test}@sys.usr # register new user with name `test`\nOr just: test@sys.usr";


fn auth(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(Usr, Option<String>)>, KernErr>> {
    thread!({
        // test
        if let Some(_ath) = read_async!(msg, ath, orig, kern)?.and_then(|(u, _)| u.as_str()) {
            return Usr::new(&_ath, &mut kern.lock()).map(|(usr, out)| Some((usr, Some(out))))
        }

        if let Some(_ath) = as_map_find_async!(msg, "ath", ath, orig, kern)?.and_then(|(u, _)| u.as_str()) {
            if let Some(pub_key) = as_map_find_async!(msg, "pub", ath, orig, kern)?.and_then(|(u, _)| u.as_str()) {
                if let Some(priv_key) = as_map_find_async!(msg, "priv", ath, orig, kern)?.and_then(|(u, _)| u.as_str()) {
                    // {ath:test pub:.. priv:..}
                    return Ok(Some((Usr::login(&_ath, &priv_key, &pub_key)?, None)))
                } else {
                    // {ath:test pub:..}
                    return Ok(Some((Usr::guest(&_ath, &pub_key)?, None)))
                }
            } else {
                // {ath:test}
                return Usr::new(&_ath, &mut kern.lock()).map(|(usr, out)| Some((usr, Some(out))))
            }
        }

        Ok(None)
    })
}

pub fn usr_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        if let Some((usr, out)) = thread_await!(auth(Rc::new(msg.ath.clone()), msg.msg.clone(), msg.msg.clone(), kern))? {
            kern.lock().reg_usr(usr.clone())?;
            writeln!(kern.lock().drv.cli, "INFO vnix:sys.usr: user `{}` registered", usr).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
            yield;

            if let Some(out) = out {
                writeln!(kern.lock().drv.cli, "WARN vnix:sys.usr: please, remember this account and save it anywhere {}", out).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
                yield;

                let m = Unit::map(&[
                    (Unit::str("msg"), Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0),
                ]);
                return kern.lock().msg(&usr.name, m).map(|msg| Some(msg));
            }

            return kern.lock().msg(&usr.name, msg.msg).map(|msg| Some(msg))
        }
        Ok(Some(msg))
    })
}
