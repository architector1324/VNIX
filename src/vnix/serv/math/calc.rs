use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use spin::Mutex;
use alloc::boxed::Box;

use crate::driver::DrvErr;

use crate::{thread, thread_await, read_async, as_map_find_async};
use crate::vnix::utils;

use crate::vnix::core::msg::Msg;
use crate::vnix::core::task::ThreadAsync;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::unit::{Unit, SchemaPair, SchemaUnit, Schema, Or};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "math.calc";
pub const SERV_HELP: &'static str = "Service for integer mathematical computation\nExample: {sum:[1 2 3]}@math.calc";


fn single_op_int_calc(op: &str, v: i32) -> Option<i32> {
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
                let v = thread_await!(single_op_int(ath.clone(), orig.clone(), Rc::new(v), kern))?;

                return Ok(op.and_then(|op| single_op_int_calc(&op, v?)))
            }
        }

        let ops = ["neg", "abs", "inc", "dec", "sqr", "sqrt", "fac", "log"];
        for op in ops {
            if let Some(v) = as_map_find_async!(msg, op, ath, orig, kern)? {
                let v = thread_await!(single_op_int(ath.clone(), orig.clone(), Rc::new(v), kern))?;
                return Ok(v.and_then(|v| single_op_int_calc(op, v)));
            }
        }

        Ok(None)
    })
}


pub fn calc_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let u = Rc::new(msg.msg.clone());
        let ath = Rc::new(msg.ath.clone());

        // single operation
        if let Some(val) = thread_await!(single_op_int(ath.clone(), u.clone(), u.clone(), kern))? {
            let m = Unit::Map(vec![
                (Unit::Str("msg".into()), Unit::Int(val as i32))]
            );

            let _msg = msg.msg.merge(m);
            return kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}