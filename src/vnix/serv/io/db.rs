use alloc::boxed::Box;
use alloc::vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, Schema, SchemaUnit, FromUnit};

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

impl FromUnit for DB {
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut db = DB::default();
        
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

impl ServHlr for DB {
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
