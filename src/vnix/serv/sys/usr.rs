use alloc::vec;
use alloc::string::String;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;
use crate::vnix::core::unit::Unit;
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


impl ServHlr for User {
    fn inst(msg: Msg, _serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = User::default();

        let mut ath = None;
        let mut pub_key = None;
        let mut priv_key = None;

        // config instance
        msg.msg.find_str(&mut vec!["ath".into()].iter()).map(|s| ath.replace(s));
        msg.msg.find_str(&mut vec!["pub".into()].iter()).map(|s| pub_key.replace(s));
        msg.msg.find_str(&mut vec!["priv".into()].iter()).map(|s| priv_key.replace(s));

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

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some(act) = &self.act {
            let (usr, out) = match act {
                UserAct::Reg{ath} => Usr::new(ath, serv.kern)?,
                UserAct::Guest {ath, pub_key} => (Usr::guest(ath, pub_key)?, String::new()),
                UserAct::Login {ath, pub_key, priv_key} => (Usr::login(ath, priv_key, pub_key)?, String::new())
            };

            serv.kern.reg_usr(usr.clone())?;
            writeln!(serv.kern.cli, "INFO vnix:sys.usr: user `{}` registered", usr).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

            if !out.is_empty() {
                writeln!(serv.kern.cli, "WARN vnix:sys.usr: please, remember this account and save it anywhere {}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

                let m = Unit::Map(vec![
                    (Unit::Str("msg".into()), Unit::parse(out.chars(), serv.kern)?.0),
                ]);
    
                return Ok(Some(serv.kern.msg(&msg.ath, m)?))
            }
        }

        Ok(Some(msg))
    }
}
