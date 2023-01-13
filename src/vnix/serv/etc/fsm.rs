use core::ops::Deref;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::driver::CLIErr;
use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::Serv;
use crate::vnix::core::kern::{KernErr, Kern};


#[derive(Debug)]
struct EventTableOut {
    state: Unit,
    msg: Unit
}

#[derive(Debug)]
struct EventTableEntry {
    ev: Unit,
    out: EventTableOut
}

#[derive(Debug)]
struct EventTable {
    state: Unit,
    table: Vec<EventTableEntry>
}

#[derive(Debug)]
pub struct FSM {
    state: Unit,
    table: Vec<EventTable>
}

impl Default for FSM {
    fn default() -> Self {
        FSM {
            state: Unit::None,
            table: Vec::new()
        }
    }
}

impl FSM {

}

impl Serv for FSM {
    fn inst(msg: Msg, kern: &mut Kern) -> Result<(Self, Msg), KernErr> {
        let mut inst = FSM::default();

        // config instance
        msg.msg.find_map(&mut vec!["fsm".into()].iter()).map(|m| {
            let u = Unit::Map(m);

            u.find_unit(&mut vec!["state".into()].iter()).map(|u| {
                inst.state = u;
            });

            if let Unit::Map(m) = u {
                inst.table = m.iter().filter_map(|(u0, u1)| Some((u0, u1.as_map()?)))
                    .map(|(state, m)| {
                        let table = m.iter().filter_map(|(ev, ent)| Some((ev, ent.as_pair()?)))
                            .map(|(ev, ent)| {
                                let out = EventTableOut {
                                    state: ent.0.deref().clone(),
                                    msg: ent.1.deref().clone(),
                                };

                                EventTableEntry {
                                    ev: ev.clone(),
                                    out
                                }
                            }).collect::<Vec<_>>();

                        EventTable {
                            state: state.clone(),
                            table
                        }
                    }).collect::<Vec<_>>();
            }
        });

        writeln!(kern.cli, "DEBG vnix:fsm: {:?}", inst).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        Ok(None)
    }
}
