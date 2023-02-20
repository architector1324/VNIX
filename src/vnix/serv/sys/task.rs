use alloc::vec;
use spin::Mutex;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::kern::Kern;
use crate::vnix::core::task::{TaskLoop};
use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic, ServHlrAsync};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaUnit, Schema, SchemaOr, Or, SchemaSeq, SchemaStr, SchemaMapSecondRequire, SchemaStream};


pub struct Task {
    name: Option<String>,
    task: Option<TaskLoop>
}

impl Default for Task {
    fn default() -> Self {
        Task{name: None, task: None}
    }
}

impl FromUnit for Task {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = Task::default();

        let schm = SchemaMapSecondRequire(
            SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit),
            SchemaMapSecondRequire(
                SchemaMapEntry(Unit::Str("name".into()), SchemaStr),
                SchemaOr(
                    SchemaMapEntry(
                        Unit::Str("task".into()),
                        SchemaOr(
                            SchemaStr,
                            SchemaSeq(SchemaStr)
                        )
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

        schm.find_loc(u).map(|(msg, (name, or))| {
            let msg = msg.and_then(|msg| Some(Unit::Map(msg.as_map()?))).unwrap_or(u.clone());
            inst.name = name;

            inst.task = match or {
                Or::First(or) =>
                    match or {
                        Or::First(serv) => Some(TaskLoop::Chain{
                            msg,
                            chain: vec![serv]
                        }),
                        Or::Second(chain) => Some(TaskLoop::Chain{
                            msg,
                            chain
                        }),
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
        });

        Some(inst)
    }
}

impl ServHlr for Task {
    fn help<'a>(self, ath: String, topic: ServHelpTopic, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
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
        ServHlrAsync(Box::new(hlr))
    }

    fn handle<'a>(self, msg: Msg, _serv: Serv, kern: &'a Mutex<Kern>) -> ServHlrAsync<'a> {
        let hlr = move || {
            if let Some(task) = self.task {
                let id = kern.lock().reg_task(&msg.ath, &self.name.unwrap_or("sys.task".into()), task)?;
                let msg;

                loop {
                    if let Some(_msg) = kern.lock().get_task_result(id) {
                        msg = _msg;
                        break;
                    }
                    yield;
                }

                let schm = SchemaMapEntry(Unit::Str("msg".into()), SchemaUnit);

                if let Some(msg) = msg {
                    if let Some(out) = schm.find_loc(&msg.msg) {
                        let u = Unit::Map(vec![
                            (Unit::Str("msg".into()), out)
                        ]);

                        return kern.lock().msg(&msg.ath, u).map(|msg| Some(msg));
                    }
                }

                return Ok(None);
            }

            Ok(Some(msg))
        };
        ServHlrAsync(Box::new(hlr))
    }
}
