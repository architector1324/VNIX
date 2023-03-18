use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use num::BigInt;

use spin::Mutex;

use alloc::vec;
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::vnix::utils::Maybe;
use crate::{thread, thread_await, as_map_find_async, as_async, maybe};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, Int, UnitNew, UnitAs, UnitReadAsyncI};


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

fn single_op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Int, Rc<String>), KernErr>> {
    thread!({
        // val
        if let Some((val, ath)) = as_async!(msg, as_int_big, ath, orig, kern)? {
            return Ok(Some((Int(val), ath)))
        }

        // (op val)
        if let Some(((op, val), ath)) = as_async!(msg, as_pair, ath, orig, kern)? {
            let (op, ath) = maybe!(as_async!(op, as_str, ath, orig, kern));
            let (val, ath) = maybe!(thread_await!(op_int(ath.clone(), orig.clone(), val, kern)));

            return Ok(calc_single_op_int(&op, val).map(|v| (v, ath)))
        }

        // {<op>:<val>}
        let ops = ["neg", "abs", "inc", "dec", "sqr", "sqrt", "fac", "log"];
        for op in ops {
            if let Some((val, ath)) = as_map_find_async!(msg, op, ath, orig, kern)? {
                let (val, ath) = maybe!(thread_await!(op_int(ath.clone(), orig.clone(), val, kern)));
                return Ok(calc_single_op_int(&op, val).map(|v| (v, ath)))
            }
        }
        Ok(None)
    })
}

fn multi_args_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Vec<Int>, Rc<String>), KernErr>> {
    thread!({
        // (v0 v1)
        if let Some(((v0, v1), ath)) = as_async!(msg, as_pair, ath, orig, kern)? {
            let (v0, ath) = maybe!(thread_await!(op_int(ath.clone(), orig.clone(), v0, kern)));
            let (v1, ath) = maybe!(thread_await!(op_int(ath.clone(), orig.clone(), v1, kern)));
    
            return Ok(Some((vec![v0, v1], ath)))
        }
    
        // [v ..]
        if let Some((lst, mut ath)) = as_async!(msg, as_list, ath, orig, kern)? {
            let mut vals = Vec::new();
            for v in Rc::unwrap_or_clone(lst) {
                let (v, _ath) = maybe!(thread_await!(op_int(ath.clone(), orig.clone(), v, kern)));
                vals.push(v);
    
                ath = _ath;
                yield;
            }
            return Ok(Some((vals, ath)))
        }
        Ok(None)
    })
}

fn multi_op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Int, Rc<String>), KernErr>> {
    thread!({
        // (op (v0 v1)) | (op [v ..])
        if let Some(((op, args), ath)) = as_async!(msg, as_pair, ath, orig, kern)? {
            let (op, ath) = maybe!(as_async!(op, as_str, ath, orig, kern));
            let (args, ath) = maybe!(thread_await!(multi_args_int(ath.clone(), orig.clone(), args, kern)));

            return Ok(calc_multi_op_int(&op, args).map(|v| (v, ath)))
        }

        let ops = ["sum", "sub", "pow", "mul", "div", "mod"];
        for op in ops {
            if let Some((args, ath)) = as_map_find_async!(msg, op, ath, orig, kern)? {
                let (args, ath) = maybe!(thread_await!(multi_args_int(ath.clone(), orig.clone(), args, kern)));
                return Ok(calc_multi_op_int(&op, args).map(|v| (v, ath)))
            }
        }
        Ok(None)
    })
}

fn op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> ThreadAsync<Maybe<(Int, Rc<String>), KernErr>> {
    thread!({
        // single operation
        if let Some((val, ath)) = thread_await!(single_op_int(ath.clone(), orig.clone(), msg.clone(), kern))? {
            return Ok(Some((val, ath)))
        }

        // multiple operands operation
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
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::int_share(val.0))]
            );
            writeln!(kern.lock().drv.cli, "{msg}");
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}