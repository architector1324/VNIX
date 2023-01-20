use core::ops::Deref;
use alloc::vec;
use alloc::vec::Vec;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::unit::Unit;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;


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

impl ServHlr for FSM {
    fn inst(msg: Msg, _serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = FSM::default();

        // config instance
        msg.msg.find_unit(&mut vec!["state".into()].iter()).map(|u| {
            inst.state = u;
        });

        msg.msg.find_map(&mut vec!["fsm".into()].iter()).map(|m| {
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

        // writeln!(kern.cli, "DEBG vnix:fsm: {:?}", inst).map_err(|_| KernErr::CLIErr(CLIErr::Write))?;

        Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
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

            return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
        }

        Ok(None)
    }
}