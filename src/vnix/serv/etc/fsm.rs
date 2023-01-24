use core::ops::Deref;
use alloc::vec;
use alloc::vec::Vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::{Unit, FromUnit};

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

        // config instance
        u.find_unit(&mut vec!["state".into()].iter()).map(|u| {
            inst.state = u;
        });

        u.find_map(&mut vec!["fsm".into()].iter()).map(|m| {
            let u = Unit::Map(m);

            if let Unit::Map(m) = u {
                // a:b
                let states = m.iter().filter(|(_, u1)| u1.as_pair().is_none() && u1.as_map().is_none())
                    .map(|(state, n_state)| {
                        EventTable {
                            state: state.clone(),
                            table: EventTableEntry::State(n_state.clone())
                        }
                    });
                
                inst.table.extend(states);

                // a:(b msg)
                let outs = m.iter().filter_map(|(u0, u1)| Some((u0, u1.as_pair()?)))
                    .map(|(state, out)| {
                        let out = EventOut {
                            state: out.0.deref().clone(),
                            msg: Some(out.1.deref().clone())
                        };

                        EventTable {
                            state: state.clone(),
                            table: EventTableEntry::Out(out)
                        }
                    });

                inst.table.extend(outs);
                
                // a:{msg:(b msg) ..}
                let events = m.iter().filter_map(|(u0, u1)| Some((u0, u1.as_map()?)))
                    .map(|(state, m)| {
                        let mut events = m.iter().filter_map(|(ev, out)| Some((ev, out.as_pair()?)))
                            .map(|(ev, out)| {
                                let out = EventOut {
                                    state: out.0.deref().clone(),
                                    msg: Some(out.1.deref().clone())
                                };

                                Event {
                                    ev: ev.clone(),
                                    out
                                }
                            }).collect::<Vec<_>>();
                        
                        let outs = m.iter().filter(|(_, out)| out.as_pair().is_none())
                            .map(|(ev, out)| {
                                let out = EventOut {
                                    state: out.clone(),
                                    msg: None
                                };

                                Event {
                                    ev: ev.clone(),
                                    out
                                }
                            }).collect::<Vec<_>>();

                        events.extend(outs);

                        EventTable {
                            state: state.clone(),
                            table: EventTableEntry::Event(events)
                        }
                    });

                inst.table.extend(events);
            }
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

    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
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
                    let msg = msg.msg.find_unit(&mut vec!["msg".into()].iter());

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

        // writeln!(kern.cli, "DEBG vnix:fsm: {:?}", out).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

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
