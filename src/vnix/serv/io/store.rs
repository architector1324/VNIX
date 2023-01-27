use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMap, SchemaMapEntry, SchemaUnit, SchemaPair, Schema, SchemaRef, SchemaMapSecondRequire};

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};

#[derive(Debug)]
pub struct Store {
    load: Option<(Vec<String>, Vec<String>)>,
    save: Option<(Vec<String>, Unit)>
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
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut store = Store::default();

        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(
                Unit::Str("load".into()),
                SchemaRef,
            ),
            SchemaMap(
                SchemaMapEntry(
                    Unit::Str("load".into()),
                    SchemaPair(SchemaRef, SchemaRef)
                ),
                SchemaMapEntry(
                    Unit::Str("save".into()),
                    SchemaPair(SchemaRef, SchemaUnit)
                )
            )
        );

        schm.find_loc(u).map(|(load_ref, (load, save))| {
            store.save = save;

            load_ref.map(|path| {
                store.load = Some((path, vec!["msg".into()]))
            });

            load.map(|(path, to)| {
                store.load = Some((path, to))
            })
        });

        Some(store)
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

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some((path, val)) = &self.save {
            kern.db_ram.save(Unit::Ref(path.clone()), val.clone());
        }

        if let Some((path, to)) = &self.load {
            let u = kern.db_ram.load(Unit::Ref(path.clone())).ok_or(KernErr::DbLoadFault)?;
            let m = Unit::merge_ref(to.clone().into_iter(), u, msg.msg).ok_or(KernErr::DbLoadFault)?;

            return Ok(Some(kern.msg(&msg.ath, m)?));
        }

        Ok(Some(msg))
    }
}
