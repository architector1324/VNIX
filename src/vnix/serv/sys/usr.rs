use core::pin::Pin;
use core::fmt::Write;
use core::ops::{Generator, GeneratorState};

use spin::Mutex;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::{DrvErr, CLIErr};

use crate::vnix::utils::Maybe;
use crate::{thread, thread_await, as_async, as_map_find_as_async, maybe};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::user::Usr;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitNew, UnitAs, UnitParse};


pub const SERV_PATH: &'static str = "sys.usr";
pub const SERV_HELP: &'static str = "Users management service\nExample: {ath:test}@sys.usr # register new user with name `test`\nOr just: test@sys.usr";


fn auth(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Usr, Option<String>), KernErr>> {
    thread!({
        // test
        if let Some((ath, _)) = as_async!(msg, as_str, ath, orig, kern)? {
            return Usr::new(&ath, &mut kern.lock()).map(|(usr, out)| Some((usr, Some(out))))
        }

        let (_ath, ath) = maybe!(as_map_find_as_async!(msg, "ath", as_str, ath, orig, kern));

        if let Some((pub_key, _)) = as_map_find_as_async!(msg, "pub", as_str, ath, orig, kern)? {
            if let Some((priv_key, _)) = as_map_find_as_async!(msg, "priv", as_str, ath, orig, kern)? {
                // {ath:test pub:.. priv:..}
                return Ok(Some((Usr::login(&_ath, &priv_key, &pub_key)?, None)))
            }

            // {ath:test pub:..}
            return Ok(Some((Usr::guest(&_ath, &pub_key)?, None)))
        }

        // {ath:test}
        return Usr::new(&_ath, &mut kern.lock()).map(|(usr, out)| Some((usr, Some(out))))
    })
}

pub fn usr_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        if let Some((usr, out)) = thread_await!(auth(Rc::new(msg.ath.clone()), msg.msg.clone(), msg.msg.clone(), kern))? {
            kern.lock().reg_usr(usr.clone())?;
            writeln!(kern.lock(), "INFO vnix:sys.usr: user `{}` registered", usr).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
            yield;

            if let Some(out) = out {
                writeln!(kern.lock(), "WARN vnix:sys.usr: please, remember this account and save it anywhere {}", out).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
                yield;

                let msg = Unit::map(&[
                    (Unit::str("msg"), Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0),
                ]);
                return kern.lock().msg(&usr.name, msg).map(|msg| Some(msg));
            }

            return kern.lock().msg(&usr.name, msg.msg).map(|msg| Some(msg))
        }
        Ok(Some(msg))
    })
}
