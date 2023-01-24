use alloc::boxed::Box;
use alloc::vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, Schema, SchemaUnit, FromUnit};

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};


pub struct Store {
    load: Option<Unit>,
    save: Option<(Unit, Unit)>
}

impl Default for Store {
    fn default() -> Self {
        Store {
            load: None,
            save: None
        }
    }
}

impl FromUnit for Store {
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut db = Store::default();
        
        // config instance
        let mut save_path = None;
        let mut save_dat = None;

        let mut schm = Schema::Unit(SchemaUnit::Map(vec![
            (
                Schema::Value(Unit::Str("load".into())),
                Schema::Unit(SchemaUnit::Unit(&mut db.load))
            ),
            (
                Schema::Value(Unit::Str("save".into())),
                Schema::Unit(SchemaUnit::Pair((
                    Box::new(Schema::Unit(SchemaUnit::Unit(&mut save_path))),
                    Box::new(Schema::Unit(SchemaUnit::Unit(&mut save_dat))),
                )))
            ),
        ]));

        schm.find(u);

        if let Some((path, dat)) = save_path.iter().filter_map(|path| Some((path.clone(), save_dat.clone()?))).next() {
            db.save.replace((path, dat));
        }

        Some(db)
    }
}

impl ServHlr for Store {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Disk units storage service\nExample: {save:(txt.doc `Some beautiful text`) task:io.store} # save text to `txt.doc` path\n{load:txt.doc task:io.store}".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some((key, val)) = &self.save {
            kern.db_ram.save(key.clone(), val.clone());
        }

        if let Some(key) = &self.load {
            let u = if key.clone() != Unit::Str("all".into()) {
                kern.db_ram.load(key.clone()).ok_or(KernErr::DbLoadFault)?
            } else {
                Unit::Map(kern.db_ram.data.clone())
            };

            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), u)
            ]);

            return Ok(Some(kern.msg(&msg.ath, m)?));
        }

        Ok(Some(msg))
    }
}
