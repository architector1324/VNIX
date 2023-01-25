use alloc::vec;
use alloc::vec::Vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaMap, SchemaUnit, SchemaMapSeq, SchemaOr, SchemaPair, Schema, Or};

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};


#[derive(Debug, Clone)]
struct EventOut {
    state: Unit,
    msg: Option<Unit>
}

#[derive(Debug)]
struct Event {
    ev: Unit,
    out: EventOut
}

#[derive(Debug)]
enum EventTableEntry {
    Event(Vec<Event>),
    Out(EventOut),
    State(Unit)
}

#[derive(Debug)]
struct EventTable {
    state: Unit,
    table: EventTableEntry
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

impl FromUnit for FSM {
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut inst = FSM::default();

        let schm = SchemaMap(
            SchemaMapEntry(Unit::Str("state".into()), SchemaUnit),
            SchemaMapEntry(
                Unit::Str("fsm".into()),
                SchemaMapSeq(
                    SchemaUnit,
                    SchemaOr(
                        SchemaOr(
                            SchemaPair(SchemaUnit, SchemaUnit),
                            SchemaMapSeq(
                                SchemaUnit,
                                SchemaOr(
                                    SchemaPair(SchemaUnit, SchemaUnit),
                                    SchemaUnit
                                )
                            )
                        ),
                        SchemaUnit
                    )
                )
            )
        );

        schm.find(u).map(|(state, fsm)| {
            state.map(|u| inst.state = u);

            fsm.map(|fsm| inst.table = fsm.iter().map(|(state, or)| {
                let table = match or {
                    Or::Second(n_state) => EventTableEntry::State(n_state.clone()), // a:b
                    Or::First(or) => match or {
                        Or::First((n_state, msg)) => // a:(b msg)
                            EventTableEntry::Out(
                                EventOut {
                                    state: n_state.clone(),
                                    msg: Some(msg.clone())
                                }
                            ),
                        Or::Second(events) => {
                            // a:{msg:(b msg) ..}
                            let events = events.iter().map(|(ev, or)| {
                                let out = match or {
                                    Or::Second(n_state) => EventOut {
                                        state: n_state.clone(),
                                        msg: None
                                    },
                                    Or::First((n_state, msg)) => EventOut {
                                        state: n_state.clone(),
                                        msg: Some(msg.clone())
                                    }
                                };

                                Event {ev: ev.clone(), out}
                            }).collect();
                            EventTableEntry::Event(events)
                        }
                    }
                };

                EventTable {
                    state: state.clone(),
                    table
                }
            }).collect());
        });

        Some(inst)
    }
}

impl ServHlr for FSM {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Finite state machine service\nExample: {fsm:{a:(b hello) b:a} state:a task:etc.fsm} # switch state `a -> b` and get `hello` msg".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        // writeln!(kern.cli, "DEBG vnix:fsm: {:?}", self).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

        let out = self.table.iter().find(|e| e.state == self.state).map(|t| {
            match &t.table {
                EventTableEntry::State(state) => {
                    EventOut {
                        state: state.clone(),
                        msg: None
                    }
                },
                EventTableEntry::Out(out) => {
                    EventOut {
                        state: out.state.clone(),
                        msg: out.msg.clone()
                    }
                },
                EventTableEntry::Event(ev) => {
                    let msg = SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit).find(&msg.msg);

                    if let Some(msg) = msg {
                        if let Some(out) = ev.iter().find(|e| e.ev == msg).map(|e| &e.out) {
                            return EventOut {
                                state: out.state.clone(),
                                msg: out.msg.clone()
                            }
                        }
                    }

                    EventOut {
                        state: self.state.clone(),
                        msg: None
                    }
                }
            }
        });

        if let Some(out) = out {
            let mut m = vec![
                (Unit::Str("state".into()), out.state),
            ];

            if let Some(msg) = out.msg {
                m.push(
                    (Unit::Str("msg".into()), msg),
                );
            }

            return Ok(Some(kern.msg(&msg.ath, Unit::Map(m))?))
        }

        Ok(None)
    }
}
