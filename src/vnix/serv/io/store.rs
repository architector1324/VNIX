use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaUnit, SchemaPair, Schema, SchemaRef, SchemaStr, SchemaOr, SchemaSeq, Or, SchemaMapFirstRequire, SchemaMapRequire};

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};


#[derive(Debug, Clone)]
struct GetSize {
    from: Vec<String>,
    to: Vec<String>
}

#[derive(Debug, Clone)]
struct Load {
    from: Vec<String>,
    to: Vec<String>
}

#[derive(Debug, Clone)]
struct Save {
    msg: Unit,
    to: Vec<String>
}

#[derive(Debug, Clone)]
enum Act {
    GetSize(GetSize),
    Load(Load),
    Save(Save)
}

#[derive(Debug)]
pub struct Store {
    act: Option<Vec<Act>>
}

impl Default for Store {
    fn default() -> Self {
        Store {
            act: None
        }
    }
}

impl Act {
    fn act(&self, kern: &mut Kern) -> Result<Option<Unit>, KernErr> {
        match self {
            Act::GetSize(size) => {
                let u = kern.ram_store.load(Unit::Ref(size.from.clone())).ok_or(KernErr::DbLoadFault)?;
                let u = Unit::Int(u.size() as i32);
                let m = Unit::merge_ref(size.to.clone().into_iter(), u, Unit::Map(Vec::new())).ok_or(KernErr::DbLoadFault)?;

                Ok(Some(m))
            }
            Act::Load(load) => {
                let u = kern.ram_store.load(Unit::Ref(load.from.clone())).ok_or(KernErr::DbLoadFault)?;
                let m = Unit::merge_ref(load.to.clone().into_iter(), u, Unit::Map(Vec::new())).ok_or(KernErr::DbLoadFault)?;

                Ok(Some(m))
            },
            Act::Save(save) => {
                kern.ram_store.save(Unit::Ref(save.to.clone()), save.msg.clone());
                Ok(None)
            }
        }
    }
}

impl FromUnit for GetSize {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapFirstRequire(
            SchemaMapEntry(Unit::Str("get.size".into()), SchemaRef),
            SchemaMapEntry(Unit::Str("out".into()), SchemaRef)
        );

        schm.find_deep(glob, u).map(|(from, to)| {
            GetSize {
                from,
                to: to.unwrap_or(vec!["msg".into()])
            }
        })
    }
}

impl FromUnit for Load {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapFirstRequire(
            SchemaMapEntry(Unit::Str("load".into()), SchemaRef),
            SchemaMapEntry(Unit::Str("out".into()), SchemaRef)
        );

        schm.find_deep(glob, u).map(|(from, to)| {
            Load {
                from,
                to: to.unwrap_or(vec!["msg".into()])
            }
        })
    }
}

impl FromUnit for Save {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapRequire(
            SchemaMapEntry(Unit::Str("save".into()), SchemaUnit),
            SchemaMapEntry(Unit::Str("out".into()), SchemaRef)
        );

        schm.find_deep(glob, u).map(|(msg, to)| {
            Save {
                msg,
                to
            }
        })
    }
}

impl FromUnit for Act {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaPair(
                SchemaStr,
                SchemaRef
            ),
            SchemaUnit
        );

        schm.find_deep(glob, u).and_then(|or| {
            match or {
                Or::First((act, path)) => {
                    match act.as_str() {
                        "get.size" => Some(Act::GetSize(GetSize {
                            from: path,
                            to: vec!["msg".into()]
                        })),
                        "load" => Some(Act::Load(
                            Load {
                                from: path,
                                to: vec!["msg".into()]
                            }
                        )),
                        "save" => Some(Act::Save(
                            Save {
                                msg: Unit::find_ref(vec!["msg".into()].into_iter(), glob)?,
                                to: path
                            }
                        )),
                        _ => None
                    }
                },
                Or::Second(u) => {
                    if let Some(size) = GetSize::from_unit(glob, &u) {
                        return Some(Act::GetSize(size));
                    }

                    if let Some(load) = Load::from_unit(glob, &u) {
                        return Some(Act::Load(load));
                    }

                    if let Some(save) = Save::from_unit(glob, &u) {
                        return Some(Act::Save(save));
                    }
                    None
                }
            }
        })
    }
}

impl FromUnit for Store {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut store = Store::default();

        let schm = SchemaOr(
            SchemaMapEntry(
                Unit::Str("store".into()),
                SchemaOr(
                    SchemaSeq(SchemaUnit),
                    SchemaUnit
                )
            ),
            SchemaUnit
        );

        schm.find_loc(u).map(|or| {
            let lst = match or {
                Or::First(or) => 
                    match or {
                        Or::First(seq) => seq,
                        Or::Second(act) => vec![act]
                    },
                Or::Second(act) => vec![act]
            };

            let acts = lst.into_iter().filter_map(|act| Act::from_unit(u, &act));

            acts.for_each(|act| {
                match store.act.as_mut() {
                    Some(acts) => acts.push(act),
                    None => store.act = Some(vec![act]),
                }
            });
        });

        Some(store)
    }
}

impl ServHlr for Store {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Disk units storage service\nExample: {store:{save:`Some beautiful text` out:txt.doc} task:io.store} # save text to `txt.doc` path\n{load:txt.doc task:io.store}".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        let mut out_u: Option<Unit> = None;

        if let Some(acts) = self.act.clone() {
            for act in acts {
                act.act(kern)?.map(|u| {
                    out_u = out_u.clone().map_or(Some(u.clone()), |out_u| Some(out_u.merge(u)))
                });
            }
        }

        if let Some(u) = out_u {
            return Ok(Some(kern.msg(&msg.ath, u)?));
        }

        Ok(Some(msg))
    }
}
