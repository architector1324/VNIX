use alloc::vec::Vec;
use alloc::vec;

use crate::vnix::core::msg::Msg;

use crate::vnix::core::serv::{Serv, ServHlr};
use crate::vnix::core::kern::KernErr;
use crate::vnix::core::unit::Unit;


pub enum Operation {
    Neg(i32),
    Abs(i32),
    Inc(i32),
    Dec(i32),
    Sqr(i32),
    Sqrt(i32),
    Fac(i32),
    Sum(Vec<i32>),
    Sub(Vec<i32>),
    Mul(Vec<i32>),
    Div(Vec<i32>),
    Mod(Vec<i32>),
    Pow(Vec<i32>),
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

impl ServHlr for Int {
    fn inst(msg: Msg, _serv: &mut Serv) -> Result<(Self, Msg), KernErr> {
        let mut inst = Int::default();

        // config instance
        msg.msg.find_int(&mut vec!["neg".into()].iter()).map(|v| inst.op = Some(Operation::Neg(v)));
        msg.msg.find_int(&mut vec!["abs".into()].iter()).map(|v| inst.op = Some(Operation::Abs(v)));
        msg.msg.find_int(&mut vec!["inc".into()].iter()).map(|v| inst.op = Some(Operation::Inc(v)));
        msg.msg.find_int(&mut vec!["dec".into()].iter()).map(|v| inst.op = Some(Operation::Dec(v)));
        msg.msg.find_int(&mut vec!["sqr".into()].iter()).map(|v| inst.op = Some(Operation::Sqr(v)));
        msg.msg.find_int(&mut vec!["sqrt".into()].iter()).map(|v| inst.op = Some(Operation::Sqrt(v)));
        msg.msg.find_int(&mut vec!["fac".into()].iter()).map(|v| inst.op = Some(Operation::Fac(v)));

        msg.msg.find_pair(&mut vec!["sum".into()].iter())
            .filter(|(u0, u1)| u0.as_int().is_some() && u1.as_int().is_some())
            .map(|(u0, u1)| inst.op = Some(Operation::Sum(vec![u0.as_int().unwrap(), u1.as_int().unwrap()])));

        msg.msg.find_list(&mut vec!["sum".into()].iter()).map(|lst| {
            let out = lst.iter().filter_map(|u| u.as_int()).collect();
            inst.op = Some(Operation::Sum(out));
        });

        msg.msg.find_pair(&mut vec!["sub".into()].iter())
            .filter(|(u0, u1)| u0.as_int().is_some() && u1.as_int().is_some())
            .map(|(u0, u1)| inst.op = Some(Operation::Sub(vec![u0.as_int().unwrap(), u1.as_int().unwrap()])));

        msg.msg.find_list(&mut vec!["sub".into()].iter()).map(|lst| {
                let out = lst.iter().filter_map(|u| u.as_int()).collect();
                inst.op = Some(Operation::Sub(out));
            });

        msg.msg.find_pair(&mut vec!["mul".into()].iter())
            .filter(|(u0, u1)| u0.as_int().is_some() && u1.as_int().is_some())
            .map(|(u0, u1)| inst.op = Some(Operation::Mul(vec![u0.as_int().unwrap(), u1.as_int().unwrap()])));

        msg.msg.find_list(&mut vec!["mul".into()].iter()).map(|lst| {
                let out = lst.iter().filter_map(|u| u.as_int()).collect();
                inst.op = Some(Operation::Mul(out));
            });

        msg.msg.find_pair(&mut vec!["div".into()].iter())
            .filter(|(u0, u1)| u0.as_int().is_some() && u1.as_int().is_some())
            .map(|(u0, u1)| inst.op = Some(Operation::Div(vec![u0.as_int().unwrap(), u1.as_int().unwrap()])));

        msg.msg.find_list(&mut vec!["div".into()].iter()).map(|lst| {
                let out = lst.iter().filter_map(|u| u.as_int()).collect();
                inst.op = Some(Operation::Div(out));
            });

        msg.msg.find_pair(&mut vec!["mod".into()].iter())
            .filter(|(u0, u1)| u0.as_int().is_some() && u1.as_int().is_some())
            .map(|(u0, u1)| inst.op = Some(Operation::Mod(vec![u0.as_int().unwrap(), u1.as_int().unwrap()])));

        msg.msg.find_list(&mut vec!["mod".into()].iter()).map(|lst| {
                let out = lst.iter().filter_map(|u| u.as_int()).collect();
                inst.op = Some(Operation::Mod(out));
            });

        msg.msg.find_pair(&mut vec!["pow".into()].iter())
            .filter(|(u0, u1)| u0.as_int().is_some() && u1.as_int().is_some())
            .map(|(u0, u1)| inst.op = Some(Operation::Pow(vec![u0.as_int().unwrap(), u1.as_int().unwrap()])));

        msg.msg.find_list(&mut vec!["pow".into()].iter()).map(|lst| {
            let out = lst.iter().filter_map(|u| u.as_int()).collect();
            inst.op = Some(Operation::Pow(out));
        });

        return Ok((inst, msg))
    }

    fn handle(&self, msg: Msg, serv: &mut Serv) -> Result<Option<Msg>, KernErr> {
        if let Some(ref op) = self.op {
            if let Operation::Neg(v) = op {
                let m = vec![
                    (Unit::Str("msg".into()), Unit::Int(-v)),
                ];
    
                return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
            }

            if let Operation::Abs(v) = op {
                let m = vec![
                    (Unit::Str("msg".into()), Unit::Int(v.abs())),
                ];
    
                return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
            }

            if let Operation::Inc(v) = op {
                let m = vec![
                    (Unit::Str("msg".into()), Unit::Int(v + 1)),
                ];
    
                return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
            }
    
            if let Operation::Dec(v) = op {
                let m = vec![
                    (Unit::Str("msg".into()), Unit::Int(v - 1)),
                ];
    
                return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
            }

            if let Operation::Sqr(v) = op {
                let m = vec![
                    (Unit::Str("msg".into()), Unit::Int(v * v)),
                ];
    
                return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
            }

            if let Operation::Sqrt(v) = op {
                let m = vec![
                    (Unit::Str("msg".into()), Unit::Int(libm::truncf(libm::sqrtf(*v as f32)) as i32)),
                ];
    
                return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
            }

            if let Operation::Fac(v) = op {
                let m = vec![
                    (Unit::Str("msg".into()), Unit::Int((1..=*v).product())),
                ];
    
                return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
            }

            if let Operation::Sum(lst) = op {
                let out = lst.iter().cloned().reduce(|a, b| a + b);

                if let Some(out) = out {
                    let m = vec![
                        (Unit::Str("msg".into()), Unit::Int(out)),
                    ];
                    return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
                }
            }

            if let Operation::Sub(lst) = op {
                let out = lst.iter().cloned().reduce(|a, b| a - b);

                if let Some(out) = out {
                    let m = vec![
                        (Unit::Str("msg".into()), Unit::Int(out)),
                    ];
                    return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
                }
            }

            if let Operation::Pow(lst) = op {
                let out = lst.iter().cloned().reduce(|a, b| a.pow(b as u32));

                if let Some(out) = out {
                    let m = vec![
                        (Unit::Str("msg".into()), Unit::Int(out)),
                    ];
                    return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
                }
            }

            if let Operation::Mul(lst) = op {
                let out = lst.iter().cloned().reduce(|a, b| a * b);

                if let Some(out) = out {
                    let m = vec![
                        (Unit::Str("msg".into()), Unit::Int(out)),
                    ];
                    return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
                }
            }

            if let Operation::Div(lst) = op {
                let out = lst.iter().cloned().reduce(|a, b| a / b);

                if let Some(out) = out {
                    let m = vec![
                        (Unit::Str("msg".into()), Unit::Int(out)),
                    ];
                    return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
                }
            }

            if let Operation::Mod(lst) = op {
                let out = lst.iter().cloned().reduce(|a, b| a % b);

                if let Some(out) = out {
                    let m = vec![
                        (Unit::Str("msg".into()), Unit::Int(out)),
                    ];
                    return Ok(Some(serv.kern.msg(&msg.ath, Unit::Map(m))?))
                }
            }
        }

        return Ok(Some(msg))
    }
}
