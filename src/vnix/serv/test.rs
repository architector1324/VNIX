use core::ops::Generator;

use crate::{vnix::core::{serv::{ServHlr, Serv, ServHelpTopic}, kern::{Kern, KernErr}, msg::Msg, unit::{FromUnit, Unit, SchemaUnit, Schema}}, driver::CLIErr};
use spin::Mutex;


pub struct Dumb {
    msg: Option<Unit>
}

impl Default for Dumb {
    fn default() -> Self {
        Dumb {msg: None}
    }
}

impl FromUnit for Dumb {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let schm = SchemaUnit;
        schm.find_loc(u).map(|u| {
            Dumb{msg: Some(u)}
        })
    }
}

impl ServHlr for Dumb {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &Mutex<Kern>) -> Result<Msg, KernErr> {
        unimplemented!()
    }

    fn handle<'a>(self, msg: Msg, _serv: Serv, kern: &'a Mutex<Kern>) -> impl Generator<Yield = (), Return = Result<Option<Msg>, KernErr>> + 'a {
        move || {
            if let Some(msg) = self.msg {
                loop {
                    writeln!(kern.lock().drv.cli, "test: {msg}").map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    yield;
                }
            }
            Ok(None)
        }
    }
}
