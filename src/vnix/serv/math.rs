use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr, ServHelpTopic};
use crate::vnix::core::kern::{KernErr, Kern};
use crate::vnix::core::unit::{Unit, FromUnit};


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
    op: Option<Operation>
}

impl Default for Int {
    fn default() -> Self {
        Int {
            op: None
        }
    }
}

impl Int {
    fn find_single_op_with(path: Vec<String>, act: SingleOpAct, u: &Unit) -> Option<Operation> {
        let op = u.find_int(&mut path.iter()).map(|v| {
            Operation::Single(
                SingleOp {
                    act: act.clone(),
                    op: Operand::Int(v)
                }
            )
        });

        if op.is_some() {
            return op;
        }

        let op = u.find_map(&mut path.iter()).map(|m| {
            Some(
                Operation::Single(
                    SingleOp {
                        act: act,
                        op: Operand::Operation(Box::new(Int::find_op(&Unit::Map(m))?))
                    }
                )
            )
        }).flatten();

        op
    }

    fn find_single_op(u: &Unit) -> Option<Operation> {
        let ops = vec![
            (vec!["neg".into()], SingleOpAct::Neg),
            (vec!["abs".into()], SingleOpAct::Abs),
            (vec!["inc".into()], SingleOpAct::Inc),
            (vec!["dec".into()], SingleOpAct::Dec),
            (vec!["sqr".into()], SingleOpAct::Sqr),
            (vec!["sqrt".into()], SingleOpAct::Sqrt),
            (vec!["fac".into()], SingleOpAct::Fac),
            (vec!["log".into()], SingleOpAct::Log),
        ];

        ops.iter().find_map(|(path, act)| Int::find_single_op_with(path.clone(), act.clone(), u))
    }

    fn find_multi_op_with(path: Vec<String>, act: MultiOpAct, u: &Unit) -> Option<Operation> {
        let op = u.find_pair(&mut path.iter()).map(|(u0, u1)| {
            let v0 = match u0 {
                Unit::Int(v) => Operand::Int(v),
                Unit::Map(m) => Operand::Operation(Box::new(Int::find_op(&Unit::Map(m))?)),
                _ => return None
            };

            let v1 = match u1 {
                Unit::Int(v) => Operand::Int(v),
                Unit::Map(m) => Operand::Operation(Box::new(Int::find_op(&Unit::Map(m))?)),
                _ => return None
            };

            Some(Operation::Multi(
                MultiOp {
                    act: act.clone(),
                    op: vec![v0, v1]
                }
            ))
        }).flatten();

        if op.is_some() {
            return op;
        }

        let op = u.find_list(&mut path.iter()).map(|lst| {
            let lst = lst.iter().cloned().map(|u| {
                match u {
                    Unit::Int(v) => Some(Operand::Int(v)),
                    Unit::Map(m) => Some(Operand::Operation(Box::new(Int::find_op(&Unit::Map(m))?))),
                    _ => return None
                }
            }).filter_map(|v| v);

            Operation::Multi(
                MultiOp {
                    act: act.clone(),
                    op: lst.collect()
                }
            )
        });

        op
    }

    fn find_multi_op(u: &Unit) -> Option<Operation> {
        let ops = vec![
            (vec!["sum".into()], MultiOpAct::Sum),
            (vec!["sub".into()], MultiOpAct::Sub),
            (vec!["mul".into()], MultiOpAct::Mul),
            (vec!["div".into()], MultiOpAct::Div),
            (vec!["mod".into()], MultiOpAct::Mod),
            (vec!["pow".into()], MultiOpAct::Pow)
        ];

        ops.iter().find_map(|(path, act)| Int::find_multi_op_with(path.clone(), act.clone(), u))
    }

    fn find_op(u: &Unit) -> Option<Operation> {
        let op = Int::find_single_op(u);
        if op.is_some() {
            return op;
        }

        Int::find_multi_op(u)
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
    fn from_unit(u: &Unit) -> Option<Self> {
        let mut inst = Int::default();

        // config instance
        inst.op = Int::find_op(u);

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

    fn handle(&self, msg: Msg, _serv: &mut Serv, kern: &mut Kern) -> Result<Option<Msg>, KernErr> {
        if let Some(ref op) = self.op {
            let out = Int::calc_op(op);

            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), Unit::Int(out)),
            ]);

            return Ok(Some(kern.msg(&msg.ath, m)?))
        }

        return Ok(Some(msg))
    }
}
