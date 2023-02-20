use alloc::vec;
use spin::Mutex;
use alloc::boxed::Box;
use alloc::string::String;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic, ServHlrAsync};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMap, SchemaMapEntry, SchemaStr, SchemaMapSecondRequire, Schema, SchemaOr, Or};
use crate::vnix::core::user::Usr;


#[derive(Debug)]
pub enum UserAct {
    Login {
        ath: String,
        pub_key: String,
        priv_key: String
    },
    Guest {
        ath: String,
        pub_key: String
    },
    Reg {
        ath: String
    }
}

pub struct User {
    act: Option<UserAct>
}

impl Default for User {
    fn default() -> Self {
        User {
            act: None
        }
    }
}

impl FromUnit for User {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = User::default();

        let mut ath = None;
        let mut pub_key = None;
        let mut priv_key = None;

        let schm = SchemaOr(
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("ath".into()), SchemaStr),
                SchemaMap(
                    SchemaMapEntry(Unit::Str("pub".into()), SchemaStr),
                    SchemaMapEntry(Unit::Str("priv".into()), SchemaStr),
                )
            ),
            SchemaStr
        );

        schm.find_loc(u).map(|or| {
            match or {
                Or::First((_ath, (_pub, _priv))) => {
                    ath = _ath;
                    pub_key = _pub;
                    priv_key = _priv;
                },
                Or::Second(_ath) => ath = Some(_ath)
            }
        });

        if let Some(ath) = ath {
            if let Some(pub_key) = pub_key {
                if let Some(priv_key) = priv_key {
                    inst.act = Some(UserAct::Login{ath, pub_key, priv_key})
                } else {
                    inst.act = Some(UserAct::Guest{ath: ath, pub_key})
                }
            } else {
                inst.act = Some(UserAct::Reg{ath})
            }
        }
        Some(inst)
    }
}

impl ServHlr for User {
    fn help<'a>(self, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Users management service\nExample: {ath:test}@sys.usr # register new user with name `test`\nOr just: test@sys.usr".into())
            };
    
            let m = Unit::Map(vec![(
                Unit::Str("msg".into()),
                u
            )]);
    
            let out = kern.lock().msg(ath.as_str(), m).map(|msg| Some(msg));
            yield;

            out
        };
        ServHlrAsync(Box::new(hlr))
    }

    fn handle<'a>(self, mut msg: Msg, _serv: Serv, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            if let Some(act) = self.act {
                let (usr, out) = match act {
                    UserAct::Reg{ref ath} => Usr::new(ath, &mut kern.lock())?,
                    UserAct::Guest{ref ath, ref pub_key} => (Usr::guest(ath, pub_key)?, String::new()),
                    UserAct::Login{ref ath, ref pub_key, ref priv_key} => (Usr::login(ath, priv_key, pub_key)?, String::new())
                };

                kern.lock().reg_usr(usr.clone())?;
                writeln!(kern.lock().drv.cli, "INFO vnix:sys.usr: user `{}` registered", usr).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                yield;
    
                if !out.is_empty() {
                    writeln!(kern.lock().drv.cli, "WARN vnix:sys.usr: please, remember this account and save it anywhere {}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;
                    yield;
    
                    let m = Unit::Map(vec![
                        (Unit::Str("msg".into()), Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0),
                    ]);
        
                    return kern.lock().msg(&usr.name, m).map(|msg| Some(msg));
                }
    
                msg = kern.lock().msg(&usr.name, msg.msg)?;
                yield;
            }
    
            Ok(Some(msg))
        };
        ServHlrAsync(Box::new(hlr))
    }
}
