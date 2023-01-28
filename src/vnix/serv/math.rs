use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit, SchemaMapEntry, SchemaPair, SchemaInt, Schema, SchemaOr, SchemaSeq, Or, SchemaUnit, SchemaRef};


#[derive(Debug)]
pub enum Operand {
    Int(i32),
    Operation(Box<Operation>)
}

#[derive(Debug, Clone)]
pub enum SingleOpAct {
    Neg,
    Abs,
    Inc,
    Dec,
    Sqr,
    Sqrt,
    Fac,
    Log
}

#[derive(Debug)]
pub struct SingleOp {
    act: SingleOpAct,
    op: Operand
}

#[derive(Debug, Clone)]
pub enum MultiOpAct {
    Sum,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

#[derive(Debug)]
pub struct MultiOp {
    act: MultiOpAct,
    op: Vec<Operand>
}

#[derive(Debug)]
pub enum Operation {
    Single(SingleOp),
    Multi(MultiOp)
}


pub struct Int {
    op: Option<(Operation, Vec<String>)>
}

impl Default for Int {
    fn default() -> Self {
        Int {
            op: None
        }
    }
}

impl Int {
    fn find_single_op_with(path: &str, act: SingleOpAct, glob: &Unit, u: &Unit) -> Option<Operation> {
        let schm = SchemaMapEntry(
            Unit::Str(path.into()),
            SchemaOr(SchemaInt, SchemaUnit)
        );

        schm.find_deep(glob, u).and_then(|or| {
            let op = match or {
                Or::First(v) => Operand::Int(v),
                Or::Second(u) => Operand::Operation(Box::new(Int::find_op(glob, &u)?)),
            };

            Some(Operation::Single(
                SingleOp {
                    act: act,
                    op
                }
            ))
        })
    }

    fn find_multi_op_with(path: &str, act: MultiOpAct, glob: &Unit, u: &Unit) -> Option<Operation> {
        let schm = SchemaMapEntry(
            Unit::Str(path.into()),
            SchemaOr(
                SchemaPair(
                    SchemaOr(SchemaInt, SchemaUnit),
                    SchemaOr(SchemaInt, SchemaUnit)
                ),
                SchemaSeq(SchemaOr(SchemaInt, SchemaUnit))
            )
        );

        schm.find_deep(glob, u).and_then(|or| {
            let op = match or {
                Or::First((a, b)) => {
                    let a = match a {
                        Or::First(v) => Operand::Int(v),
                        Or::Second(u) => Operand::Operation(Box::new(Int::find_op(glob, &u)?))
                    };
        
                    let b = match b {
                        Or::First(v) => Operand::Int(v),
                        Or::Second(u) => Operand::Operation(Box::new(Int::find_op(glob, &u)?))
                    };
        
                    vec![a, b]
                },
                Or::Second(seq) =>
                    seq.iter().map(|or| {
                        match or {
                            Or::First(v) => Some(Operand::Int(*v)),
                            Or::Second(u) => Some(Operand::Operation(Box::new(Int::find_op(glob, u)?)))
                        }
                    }).filter_map(|v| v).collect::<Vec<_>>()
            };

            Some(Operation::Multi(
                MultiOp {
                    act: act,
                    op
                }
            ))
        })
    }

    fn find_single_op(glob: &Unit, u: &Unit) -> Option<Operation> {
        let ops = vec![
            ("neg", SingleOpAct::Neg),
            ("abs", SingleOpAct::Abs),
            ("inc", SingleOpAct::Inc),
            ("dec", SingleOpAct::Dec),
            ("sqr", SingleOpAct::Sqr),
            ("sqrt", SingleOpAct::Sqrt),
            ("fac", SingleOpAct::Fac),
            ("log", SingleOpAct::Log),
        ];

        ops.iter().find_map(|(path, act)| Int::find_single_op_with(path.clone(), act.clone(), glob, u))
    }

    fn find_multi_op(glob: &Unit, u: &Unit) -> Option<Operation> {
        let ops = vec![
            ("sum", MultiOpAct::Sum),
            ("sub", MultiOpAct::Sub),
            ("mul", MultiOpAct::Mul),
            ("div", MultiOpAct::Div),
            ("mod", MultiOpAct::Mod),
            ("pow", MultiOpAct::Pow)
        ];

        ops.iter().find_map(|(path, act)| Int::find_multi_op_with(path.clone(), act.clone(), glob, u))
    }

    fn find_op(glob: &Unit, u: &Unit) -> Option<Operation> {
        let op = Int::find_single_op(glob, u);
        if op.is_some() {
            return op;
        }

        Int::find_multi_op(glob, u)
    }

    fn calc_op(op: &Operation) -> i32 {
        match op {
            Operation::Single(op) => Int::calc_single_op(op),
            Operation::Multi(op) => Int::calc_multi_op(op)
        }
    }

    fn calc_single_op(op: &SingleOp) -> i32 {
        let v = match op.op {
            Operand::Int(v) => v,
            Operand::Operation(ref op) => Int::calc_op(&op)
        };

        match op.act {
            SingleOpAct::Neg => -v,
            SingleOpAct::Abs => v.abs(),
            SingleOpAct::Inc => v + 1,
            SingleOpAct::Dec => v - 1,
            SingleOpAct::Sqr => v * v,
            SingleOpAct::Sqrt => libm::truncf(libm::sqrtf(v as f32)) as i32,
            SingleOpAct::Fac => (1..=v).product(),
            SingleOpAct::Log => libm::truncf(libm::logf(v as f32)) as i32
        }
    }

    fn calc_multi_op(op: &MultiOp) -> i32 {
        let lst = op.op.iter().map(|op| {
            match op {
                Operand::Int(v) => *v,
                Operand::Operation(ref op) => Int::calc_op(&op)
            }
        });

        lst.reduce(|a, b| {
            match op.act {
                MultiOpAct::Sum => a + b,
                MultiOpAct::Sub => a - b,
                MultiOpAct::Pow => a.pow(b as u32),
                MultiOpAct::Mul => a * b,
                MultiOpAct::Div => a / b,
                MultiOpAct::Mod => a % b
            }
        }).unwrap_or(0)
    }
}

impl FromUnit for Int {
    fn from_unit_loc(u: &Unit) -> Option<Self> {
        let mut inst = Int::default();

        if let Some(op) = Int::find_op(u, u) {
            inst.op = Some((op, vec!["msg".into()]));
        } else {
            let schm = SchemaMapEntry(
                Unit::Str("val".into()),
                SchemaPair(SchemaRef, SchemaUnit)
            );

            schm.find_loc(u).map(|(path, loc)| {
                if let Some(op) = Int::find_op(&u, &loc) {
                    inst.op = Some((op, path))
                }
            });
        }

        Some(inst)
    }
}

impl ServHlr for Int {
    fn help(&self, ath: &str, topic: ServHelpTopic, kern: &mut Kern) -> Result<Msg, KernErr> {
        let u = match topic {
            ServHelpTopic::Info => Unit::Str("Service for integer mathematical computation\nExample: {sum:[1 2 3] task:math.int}".into())
        };

        let m = Unit::Map(vec![(
            Unit::Str("msg".into()),
            u
        )]);

        return Ok(kern.msg(ath, m)?)
    }

    fn handle(&mut self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some((op, path)) = &self.op {
            let out = Int::calc_op(op);

            let m = Unit::merge_ref(path.clone().into_iter(), Unit::Int(out), msg.msg).ok_or(KernErr::DbLoadFault)?;

            return Ok(Some(kern.msg(&msg.ath, m)?))
        }

        return Ok(Some(msg))
    }
}
