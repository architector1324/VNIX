use alloc::vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;


pub struct DB {
    load: Option<Unit>,
    save: Option<(Unit, Unit)>
}

impl Default for DB {
    fn default() -> Self {
        DB {
            load: None,
            save: None
        }
    }
}

impl ServHlr for DB {
    fn inst(msg: Msg, _serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut db = DB::default();
        
        // config instance
        msg.msg.find_unit(&mut vec!["load".into()].iter()).map(|u| {
            db.load.replace(u)
        });

        msg.msg.find_pair(&mut vec!["save".into()].iter()).map(|p| {
            db.save.replace(p)
        });

        Ok((db, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some((key, val)) = &self.save {
            serv.kern.db_ram.save(key.clone(), val.clone());
        }

        if let Some(key) = &self.load {
            let u = if key.clone() != Unit::Str("all".into()) {
                serv.kern.db_ram.load(key.clone()).ok_or(KernErr::DbLoadFault)?
            } else {
                Unit::Map(serv.kern.db_ram.data.clone())
            };

            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), u)
            ]);

            return Ok(Some(serv.kern.msg(&msg.ath, m)?));
        }

        Ok(Some(msg))
    }
}
