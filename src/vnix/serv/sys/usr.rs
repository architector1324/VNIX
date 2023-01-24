use alloc::vec;
use alloc::string::String;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, Schema, SchemaUnit, FromUnit};
use crate::vnix::core::user::Usr;


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
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut inst = User::default();

        let mut ath = None;
        let mut pub_key = None;
        let mut priv_key = None;

        // config instance
        let mut schm = Schema::Unit(SchemaUnit::Map(vec![
            (
                Schema::Value(Unit::Str("ath".into())),
                Schema::Unit(SchemaUnit::Str(&mut ath))
            ),
            (
                Schema::Value(Unit::Str("pub".into())),
                Schema::Unit(SchemaUnit::Str(&mut pub_key))
            ),
            (
                Schema::Value(Unit::Str("priv".into())),
                Schema::Unit(SchemaUnit::Str(&mut priv_key))
            ),
        ]));

        schm.find(u);

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
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Users registration service\nExample: {ath:test task:sys.usr} # register new user with name `test`".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&self, mut msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(act) = &self.act {
            let (usr, out) = match act {
                UserAct::Reg {ath} => Usr::new(ath, kern)?,
                UserAct::Guest {ath, pub_key} => (Usr::guest(ath, pub_key)?, String::new()),
                UserAct::Login {ath, pub_key, priv_key} => (Usr::login(ath, priv_key, pub_key)?, String::new())
            };

            kern.reg_usr(usr.clone())?;
            writeln!(kern.cli, "INFO vnix:sys.usr: user `{}` registered", usr).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

            if !out.is_empty() {
                writeln!(kern.cli, "WARN vnix:sys.usr: please, remember this account and save it anywhere {}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

                let m = Unit::Map(vec![
                    (Unit::Str("msg".into()), Unit::parse(out.chars()).map_err(|e| KernErr::ParseErr(e))?.0),
                ]);
    
                return Ok(Some(kern.msg(&usr.name, m)?))
            }

            msg = kern.msg(&usr.name, msg.msg)?;
        }

        Ok(Some(msg))
    }
}
