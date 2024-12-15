use core::fmt::Write;

use alloc::rc::Rc;
use alloc::boxed::Box;
use alloc::string::String;

use spin::Mutex;
use async_trait::async_trait;

use crate::vnix::utils::Maybe;
use crate::{as_async, as_map_find_as_async, maybe};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::user::Usr;
use crate::vnix::core::task::Yield;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::driver::{DrvErr, CLIErr};
use crate::vnix::core::serv::{ServHlr, ServInfo, ServResult};
use crate::vnix::core::unit::{Unit, UnitReadAsyncI, UnitNew, UnitAs, UnitParse};


pub const SERV_PATH: &'static str = "sys.usr";

pub const SERV_HELP: &'static str = "{
    name:sys.usr
    info:`Users management service`
    tut:[
        {
            info:`Register new 'test' user`
            com:[
                test@sys.usr
                {ath:test}@sys.usr
            ]
            res:{
                ath:test
                pub:`AiOte6qwiIcJTWzLjAyA+d6pwVs4eRTi7fEqdDFy2a6z`
                priv:`AYi2fBh4vQ/aQR2qU78XlTsx3huL0dIGzIsRHKYB+ls=`
            }
        }
        {
            info:`Login 'test' guest user.\\nServices will not able to create new messages, read-only.`
            com:{
                ath:test
                pub:`AiOte6qwiIcJTWzLjAyA+d6pwVs4eRTi7fEqdDFy2a6z`
            }@sys.usr
            res:{
                ath:test
                pub:`AiOte6qwiIcJTWzLjAyA+d6pwVs4eRTi7fEqdDFy2a6z`
                priv:-
            }
        }
        {
            info:`Login 'test' user`
            com:{
                ath:test
                pub:`AiOte6qwiIcJTWzLjAyA+d6pwVs4eRTi7fEqdDFy2a6z`
                priv:`AYi2fBh4vQ/aQR2qU78XlTsx3huL0dIGzIsRHKYB+ls=`
            }@sys.usr
            res:{
                ath:test
                pub:`AiOte6qwiIcJTWzLjAyA+d6pwVs4eRTi7fEqdDFy2a6z`
                priv:`AYi2fBh4vQ/aQR2qU78XlTsx3huL0dIGzIsRHKYB+ls=`
            }
        }
    ]
    man:-
}";

pub struct UsrHlr;

impl UsrHlr {
    async fn auth(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> Maybe<(Usr, Option<String>), KernErr> {
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
    }
}

#[async_trait(?Send)]
impl ServHlr for UsrHlr {
    async fn hlr(&self, msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServResult {
        if let Some((usr, out)) = Self::auth(Rc::new(msg.ath.clone()), msg.msg.clone(), msg.msg.clone(), kern).await? {
            kern.lock().reg_usr(usr.clone())?;
            writeln!(kern.lock(), "INFO vnix:sys.usr: user `{}` registered", usr).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
            Yield::now().await;

            if let Some(out) = out {
                writeln!(kern.lock(), "WARN vnix:sys.usr: please, remember this account and save it anywhere {}", out).map_err(|_| KernErr::DrvErr(DrvErr::CLI(CLIErr::Write)))?;
                Yield::now().await;

                let msg = Unit::map(&[
                    (Unit::str("msg"), Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0),
                ]);
                return kern.lock().msg(&usr.name, msg).map(|msg| Some(msg));
            }

            return kern.lock().msg(&usr.name, msg.msg).map(|msg| Some(msg))
        }
        Ok(Some(msg))
    }
}
