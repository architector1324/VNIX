use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use num::BigInt;

use spin::Mutex;

use alloc::vec;
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::{thread, thread_await, read_async, as_map_find_async};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, Int, UnitNew, UnitAs, UnitReadAsyncI, UnitModify};


pub const SERV_PATH: &'static str = "math.calc";
pub const SERV_HELP: &'static str = "Service for integer mathematical computation\nExample: {sum:[1 2 3]}@math.calc";


fn calc_single_op_int(op: &str, v: Int) -> Option<Int> {
    let res = match op {
        "neg" => -v.0.as_ref(),
        "abs" => num::abs(Rc::unwrap_or_clone(v.0)),
        "inc" => v.0.as_ref() + 1,
        "dec" => v.0.as_ref() - 1,
        "sqr" => v.0.as_ref() * v.0.as_ref(),
        "sqrt" => v.0.sqrt(),
        "fac" => (1..=v.to_nat()?).fold(BigInt::from(1), |a, b| BigInt::from(a) * BigInt::from(b)),
        // "log" => libm::truncf(libm::logf(v as f32)) as i32,
        _ => return None
    };
    Some(Int(Rc::new(res)))
}

fn calc_multi_op_int(op: &str, vals: Vec<Int>) -> Option<Int> {
    vals.into_iter().try_reduce(|a, b| {
        let res = match op {
            "sum" => a.0.as_ref() + b.0.as_ref(),
            "sub" => a.0.as_ref() - b.0.as_ref(),
            "pow" => a.0.pow(b.to_nat()?),
            "mul" => a.0.as_ref() * b.0.as_ref(),
            "div" => a.0.as_ref() / b.0.as_ref(),
            "mod" => a.0.as_ref() % b.0.as_ref(),
            _ => return None
        };
        Some(Int(Rc::new(res)))
    }).flatten()
}

fn single_op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(Int, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((res, ath)) = read_async!(msg, ath, orig, kern)? {
            // val
            if let Some(val) = res.as_int_big() {
                return Ok(Some((Int(val), ath)))
            }

            // (op val)
            if let Some((op, v)) = msg.clone().as_pair() {
                if let Some((op, ath)) = read_async!(op, ath, orig, kern)?.and_then(|(s, ath)| Some((s.as_str()?, ath))) {
                    if let Some((v, ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v, kern))? {
                        return Ok(calc_single_op_int(&op, v).map(|v| (v, ath)))
                    }
                }
            }
        }

        let ops = ["neg", "abs", "inc", "dec", "sqr", "sqrt", "fac", "log"];
        for op in ops {
            if let Some((v, ath)) = as_map_find_async!(msg, op, ath, orig, kern)? {
                if let Some((v, ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v, kern))? {
                    return Ok(calc_single_op_int(&op, v).map(|v| (v, ath)))
                }
            }
        }

        Ok(None)
    })
}

fn multi_op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(Int, Rc<String>)>, KernErr>> {
    thread!({
        if let Some((res, ath)) = read_async!(msg, ath, orig, kern)? {
            // (op (v0 v1))
            if let Some((op, (v0, v1))) = res.clone().as_pair().into_iter().filter_map(|(u0, u1)| Some((u0, u1.as_pair()?))).next() {
                if let Some((op, ath)) = read_async!(op, ath, orig, kern)?.and_then(|(s, ath)| Some((s.as_str()?, ath))) {
                    if let Some((v0, ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v0, kern))? {
                        if let Some((v1, ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v1, kern))? {
                            return Ok(calc_multi_op_int(&op, vec![v0, v1]).map(|v| (v, ath)))
                        }
                    }
                }
            }

            // (op [v ..])
            if let Some((op, lst)) = res.as_pair().into_iter().filter_map(|(u0, u1)| Some((u0, u1.as_list()?))).next() {
                if let Some((op, mut ath)) = read_async!(op, ath, orig, kern)?.and_then(|(s, ath)| Some((s.as_str()?, ath))) {
                    let mut vals = Vec::new();
                    for v in Rc::unwrap_or_clone(lst) {
                        if let Some((v, _ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v, kern))? {
                            ath = _ath;
                            vals.push(v);
                        }
                    }

                    return Ok(calc_multi_op_int(&op, vals).map(|v| (v, ath)))
                }
            }
        }

        let ops = ["sum", "sub", "pow", "mul", "div", "mod"];
        for op in ops {
            if let Some((u, mut ath)) = as_map_find_async!(msg, op, ath, orig, kern)? {
                // (v0 v1)
                if let Some((v0, v1)) = u.clone().as_pair() {
                    if let Some((v0, ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v0, kern))? {
                        if let Some((v1, ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v1, kern))? {
                            return Ok(calc_multi_op_int(&op, vec![v0, v1]).map(|v| (v, ath)))
                        }
                    }
                }

                // [v ..]
                if let Some(lst) = u.as_list() {
                    let mut vals = Vec::new();
                    for v in Rc::unwrap_or_clone(lst) {
                        if let Some((v, _ath)) = thread_await!(op_int(ath.clone(), orig.clone(), v, kern))? {
                            ath = _ath;
                            vals.push(v);
                        }
                    }

                    return Ok(calc_multi_op_int(&op, vals).map(|v| (v, ath)))
                }
            }
        }

        Ok(None)
    })
}

fn op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<(Int, Rc<String>)>, KernErr>> {
    thread!({
        // single operation
        if let Some((val, ath)) = thread_await!(single_op_int(ath.clone(), orig.clone(), msg.clone(), kern))? {
            return Ok(Some((val, ath)))
        }

        // multiple operands opearation
        if let Some((val, ath)) = thread_await!(multi_op_int(ath, orig, msg, kern))? {
            return Ok(Some((val, ath)))
        }
        Ok(None)
    })
}

pub fn calc_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());

        if let Some((val, ath)) = thread_await!(op_int(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let m = Unit::map(&[
                (Unit::str("msg"), Unit::int_share(val.0))]
            );

            let _msg = msg.msg.merge_with(m);
            return kern.lock().msg(&ath, _msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}