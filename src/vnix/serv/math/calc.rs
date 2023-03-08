use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

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
use crate::vnix::core::unit::{Unit, SchemaPair, SchemaUnit, Schema, SchemaSeq};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "math.calc";
pub const SERV_HELP: &'static str = "Service for integer mathematical computation\nExample: {sum:[1 2 3]}@math.calc";


fn calc_single_op_int(op: &str, v: i32) -> Option<i32> {
    let res = match op {
        "neg" => -v,
        "abs" => v.abs(),
        "inc" => v + 1,
        "dec" => v - 1,
        "sqr" => v * v,
        "sqrt" => libm::truncf(libm::sqrtf(v as f32)) as i32,
        "fac" => (1..=v).product(),
        "log" => libm::truncf(libm::logf(v as f32)) as i32,
        _ => return None
    };
    Some(res)
}

fn calc_multi_op_int(op: &str, vals: Vec<i32>) -> Option<i32> {
    vals.into_iter().try_reduce(|a, b| {
        let res = match op {
            "sum" => a + b,
            "sub" => a - b,
            "pow" => a.pow(b as u32),
            "mul" => a * b,
            "div" => a / b,
            "mod" => a % b,
            _ => return None
        };
        Some(res)
    }).flatten()
}

fn single_op_int(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<i32>, KernErr>> {
    thread!({
        if let Some(res) = read_async!(msg, ath, orig, kern)? {
            // val
            if let Some(val) = res.as_int() {
                return Ok(Some(val))
            }

            // (op val)
            let schm = SchemaPair(SchemaUnit, SchemaUnit);
            if let Some((op, v)) = schm.find(&orig, &res) {
                let op = read_async!(Rc::new(op), ath, orig, kern)?.and_then(|s| s.as_str());
                let v = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v), kern))?;

                return Ok(op.and_then(|op| calc_single_op_int(&op, v?)))
            }
        }

        let ops = ["neg", "abs", "inc", "dec", "sqr", "sqrt", "fac", "log"];
        for op in ops {
            if let Some(v) = as_map_find_async!(msg, op, ath, orig, kern)? {
                let v = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v), kern))?;
                return Ok(v.and_then(|v| calc_single_op_int(op, v)));
            }
        }

        Ok(None)
    })
}

fn multi_op_int(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<i32>, KernErr>> {
    thread!({
        if let Some(res) = read_async!(msg, ath, orig, kern)? {
            // (op (v0 v1))
            let schm = SchemaPair(
                SchemaUnit,
                SchemaPair(SchemaUnit, SchemaUnit)
            );

            if let Some((op, (v0, v1))) = schm.find(&orig, &res) {
                let op = read_async!(Rc::new(op), ath, orig, kern)?.and_then(|s| s.as_str());
                let v0 = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v0), kern))?;
                let v1 = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v1), kern))?;

                return Ok(op.and_then(|op| calc_multi_op_int(&op, vec![v0?, v1?])))
            }

            // (op [v ..])
            let schm = SchemaPair(
                SchemaUnit,
                SchemaSeq(SchemaUnit)
            );

            if let Some((op, lst)) = schm.find(&orig, &res) {
                let op = read_async!(Rc::new(op), ath, orig, kern)?.and_then(|s| s.as_str());

                let mut tmp = Vec::new();
                for v in lst {
                    let v = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v), kern))?;
                    tmp.push(v);
                }

                let vals = tmp.into_iter().collect::<Option<Vec<_>>>();

                return Ok(vals.and_then(|vals| calc_multi_op_int(&op?, vals)))
            }
        }

        let ops = ["sum", "sub", "pow", "mul", "div", "mod"];
        for op in ops {
            if let Some(u) = as_map_find_async!(msg, op, ath, orig, kern)? {
                // (v0 v1)
                let schm = SchemaPair(SchemaUnit, SchemaUnit);

                if let Some((v0, v1)) = schm.find(&orig, &u) {
                    let v0 = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v0), kern))?;
                    let v1 = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v1), kern))?;

                    return Ok(v0.and_then(|v0| calc_multi_op_int(&op, vec![v0, v1?])))
                }
            
                // [v ..]
                if let Some(lst) = u.as_vec() {
                    let mut tmp = Vec::new();
                    for v in lst {
                        let v = thread_await!(op_int(ath.clone(), orig.clone(), Rc::new(v), kern))?;
                        tmp.push(v);
                    }
    
                    let vals = tmp.into_iter().collect::<Option<Vec<_>>>();
                    return Ok(vals.and_then(|vals| calc_multi_op_int(&op, vals)))
                }
            }
        }

        Ok(None)
    })
}

fn op_int(ath: Rc<String>, orig: Rc<Unit>, msg: Rc<Unit>, kern: &Mutex<Kern>) -> ThreadAsync<Result<Option<i32>, KernErr>> {
    thread!({
        // single operation
        if let Some(val) = thread_await!(single_op_int(ath.clone(), orig.clone(), msg.clone(), kern))? {
            return Ok(Some(val))
        }
    
        // multiple operands opearation
        if let Some(val) = thread_await!(multi_op_int(ath, orig, msg, kern))? {
            return Ok(Some(val))
        }
        Ok(None)
    })
}

pub fn calc_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());
        let ath = Rc::new(msg.ath.clone());

        if let Some(val) = thread_await!(op_int(ath.clone(), u.clone(), u, kern))? {
            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), Unit::Int(val as i32))]
            );

            let _msg = msg.msg.merge(m);
            return kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}