use core::ops::{Generator, GeneratorState};
use core::pin::Pin;
use core::slice::Iter;

use alloc::rc::Rc;
use spin::Mutex;

use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::task::{TaskLoop, TaskSig};
use crate::vnix::core::serv::{ServHlr, ServHelpTopic, ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaUnit, Schema, SchemaOr, Or, SchemaSeq, SchemaStr, SchemaMapSecondRequire, SchemaStream, SchemaPair, SchemaInt};


#[derive(Debug, Clone)]
enum GetRunning {
    Curr,
    All,
    Tree
}

#[derive(Debug, Clone)]
struct Run {
    name: String,
    task: Option<TaskLoop>
}

#[derive(Debug, Clone)]
struct Signal {
    id: usize,
    sig: TaskSig
}

pub type TaskActAsync<'a> = Box<dyn Generator<Yield = (), Return = Result<Option<Unit>, KernErr>> + 'a>;

pub trait TaskAct {
    fn act<'a>(self, orig: Rc<Msg>, kern: &'a Mutex<Kern>) -> TaskActAsync<'a>;
}

#[derive(Debug, Clone)]
enum Act {
    Run(Run),
    Get(GetRunning),
    Sig(Signal)
}

pub struct Task {
    act: Option<Act>
}

impl Default for Task {
    fn default() -> Self {
        Task{act: None}
    }
}

impl FromUnit for GetRunning {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaMapEntry(Unit::Str("task".into()), SchemaStr),
            SchemaStr
        );

        schm.find_deep(glob, u).and_then(|or| {
            let s = match or {
                Or::First(s) => s,
                Or::Second(s) => s
            };

            match s.as_str() {
                "get.curr" => Some(GetRunning::Curr),
                "get.all" => Some(GetRunning::All),
                "get.tree" => Some(GetRunning::Tree),
                _ => None
            }
        })
    }
}

impl FromUnit for Run {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(_glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("name".into()), SchemaStr),
                SchemaOr(
                    SchemaOr(
                        SchemaMapEntry(
                            Unit::Str("task".into()),
                            SchemaOr(
                                SchemaStr,
                                SchemaSeq(SchemaStr)
                            )
                        ),
                        SchemaMapEntry(
                            Unit::Str("task.loop".into()),
                            SchemaOr(
                                SchemaStr,
                                SchemaSeq(SchemaStr)
                            )
                        ),
                    ),
                    SchemaOr(
                        SchemaMapEntry(
                            Unit::Str("task.sim".into()),
                            SchemaSeq(SchemaStream)
                        ),
                        SchemaMapEntry(
                            Unit::Str("task.que".into()),
                            SchemaSeq(SchemaStream)
                        ),
                    )
                )
            )
        );

        schm.find_loc(u).and_then(|(msg, (name, or))| {
            let name = name.unwrap_or("sys.task".into());

            if let Some(msg) = msg.clone() {
                if let Some(m) = msg.as_map() {
                    let u = Unit::Map(m);
                    return Run::from_unit_loc(&u);
                }
            }

            let msg = msg.and_then(|msg| Some(Unit::Map(msg.as_map()?))).unwrap_or(u.clone());

            let task = match or {
                Or::First(or) =>
                    match or {
                        Or::First(or) =>
                            match or {
                                Or::First(serv) => Some(TaskLoop::Chain {
                                    msg,
                                    chain: vec![serv],
                                }),
                                Or::Second(chain) => Some(TaskLoop::Chain {
                                    msg,
                                    chain,
                                })
                            },
                        Or::Second(or) =>
                            match or {
                                Or::First(serv) => Some(TaskLoop::ChainLoop {
                                    msg,
                                    chain: vec![serv],
                                }),
                                Or::Second(chain) => Some(TaskLoop::ChainLoop {
                                    msg,
                                    chain,
                                })
                            },
                    }
                Or::Second(or) =>
                    match or {
                        Or::First(sim) => Some(TaskLoop::Sim(
                            sim.into_iter().map(|(msg, (serv, _))| (msg, serv)).collect()
                        )),
                        Or::Second(queue) => Some(TaskLoop::Queue(
                            queue.into_iter().map(|(msg, (serv, _))| (msg, serv)).collect()
                        ))
                    }
            };
            Some(Run{name, task})
        })
    }
}

impl FromUnit for Signal {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        Self::from_unit(u, u)
    }

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        let schm = SchemaOr(
            SchemaMapEntry(
                Unit::Str("task".into()),
                SchemaPair(SchemaStr, SchemaInt)
            ),
            SchemaPair(SchemaStr, SchemaInt)
        );

        schm.find_deep(glob, u).and_then(|or| {
            let (s, id) = match or {
                Or::First(p) => p,
                Or::Second(p) => p
            };

            match s.as_str() {
                "kill" => Some(Signal {
                    id: id as usize,
                    sig: TaskSig::Kill
                }),
                _ => None
            }
        })
    }
}

impl FromUnit for Task {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = Task::default();

        if let Some(get) = GetRunning::from_unit(u, u) {
            inst.act = Some(Act::Get(get));
            return Some(inst);
        }

        if let Some(sig) = Signal::from_unit(u, u) {
            inst.act = Some(Act::Sig(sig));
            return Some(inst);
        }

        if let Some(run) = Run::from_unit(u, u) {
            inst.act = Some(Act::Run(run));
            return Some(inst);
        }

        Some(inst)
    }
}

impl TaskAct for GetRunning {
    fn act<'a>(self, orig: Rc<Msg>, kern: &'a Mutex<Kern>) -> TaskActAsync<'a> {
        let hlr = move || {
            let msg = match self {
                GetRunning::Curr => Unit::Int(kern.lock().get_task_running() as i32),
                GetRunning::All => {
                    Unit::Lst(kern.lock().get_tasks_running().into_iter().map(|t| {
                        Unit::Map(vec![
                            (Unit::Str("id".into()), Unit::Int(t.id as i32)),
                            (Unit::Str("usr".into()), Unit::Str(t.usr)),
                            (Unit::Str("name".into()), Unit::Str(t.name)),
                            (Unit::Str("par.id".into()), Unit::Int(t.parent_id as i32)),
                        ])
                    }).collect())
                },
                GetRunning::Tree => {
                    let mut tasks = kern.lock().get_tasks_running();
                    tasks.sort_by(|prev, t| prev.id.cmp(&t.id));

                    if let Some(first) = tasks.get(0) {
                        fn tree(t: &crate::vnix::core::task::Task, it: Iter<crate::vnix::core::task::Task>) -> Unit {
                            let lst = it.clone().filter(|_t| _t.id != t.id && _t.parent_id == t.id).map(|_t| tree(_t, it.clone())).collect::<Vec<_>>();

                            let childs = if !lst.is_empty() {
                                Unit::Lst(lst)
                            } else {
                                Unit::None
                            };

                            Unit::Map(vec![
                                (Unit::Str("id".into()), Unit::Int(t.id as i32)),
                                (Unit::Str("usr".into()), Unit::Str(t.usr.clone())),
                                (Unit::Str("name".into()), Unit::Str(t.name.clone())),
                                (Unit::Str("chld".into()), childs)
                                ])
                        }
                        tree(first, tasks.iter())
                    } else {
                        Unit::None
                    }
                },
            };
            yield;

            let msg = orig.msg.clone().merge(Unit::Map(vec![(Unit::Str("msg".into()), msg)]));
            Ok(Some(msg))
        };
        Box::new(hlr)
    }
}

impl TaskAct for Signal {
    fn act<'a>(self, orig: Rc<Msg>, kern: &'a Mutex<Kern>) -> TaskActAsync<'a> {
        let hlr = move || {
            kern.lock().task_sig(self.id, self.sig)?;
            yield;

            Ok(Some(orig.msg.clone()))
        };
        Box::new(hlr)
    }
}

impl TaskAct for Run {
    fn act<'a>(self, orig: Rc<Msg>, kern: &'a Mutex<Kern>) -> TaskActAsync<'a> {
        let hlr = move || {
            if let Some(task) = self.task {
                let id = kern.lock().reg_task(&orig.ath, &self.name, task)?;
                let msg;

                loop {
                    if let Some(_msg) = kern.lock().get_task_result(id) {
                        msg = _msg?;
                        break;
                    }
                    yield;
                }

                let schm = SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit);

                if let Some(msg) = msg {
                    if let Some(out) = schm.find_loc(&msg.msg) {
                        let msg = Unit::Map(vec![
                            (Unit::Str("msg".into()), out)
                        ]);

                        return Ok(Some(msg))
                    }
                }

                return Ok(None);
            }

            Ok(Some(orig.msg.clone()))
        };
        Box::new(hlr)
    }
}

impl ServHlr for Task {
    fn inst(&self, msg: &Unit) -> Result<Box<dyn ServHlr>, KernErr> {
        let inst = Self::from_unit_loc(msg).ok_or(KernErr::CannotCreateServInstance)?;
        Ok(Box::new(inst))
    }

    fn help<'a>(self: Box<Self>, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            let u = match topic {
                ServHelpTopic::Info => Unit::Str("Service for run task from message\nExample: {store:(load @txt.hello) task:io.store}@sys.task".into())
            };

            let m = Unit::Map(vec![(
                Unit::Str("msg".into()),
                u
            )]);
    
            let out = kern.lock().msg(&ath, m).map(|msg| Some(msg));
            yield;

            out
        };
        Box::new(hlr)
    }

    fn handle<'a>(self: Box<Self>, msg: Msg, _serv: ServInfo, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            if let Some(act) = self.act {
                let orig = Rc::new(msg);

                let act = match act {
                    Act::Get(get) => get.act(orig.clone(), kern),
                    Act::Run(run) => run.act(orig.clone(), kern),
                    Act::Sig(sig) => sig.act(orig.clone(), kern)
                };

                let mut gen = Box::into_pin(act);
                let mut _msg = None;

                loop {
                    if let GeneratorState::Complete(res) = Pin::new(&mut gen).resume(()) {
                        if let Some(msg) = res? {
                            _msg = kern.lock().msg(&orig.ath, msg).map(|msg| Some(msg))?;
                        }
                        break;
                    }
                    yield;
                }

                Ok(_msg)
            } else {
                Ok(Some(msg))
            }
        };
        Box::new(hlr)
    }
}
