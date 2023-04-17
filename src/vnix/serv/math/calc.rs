use core::pin::Pin;
use core::ops::{Generator, GeneratorState};

use num::BigInt;

use spin::Mutex;

use alloc::vec;
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

use crate::{thread, thread_await, as_map_find_async, as_async, maybe, maybe_ok};

use crate::vnix::core::msg::Msg;
use crate::vnix::core::kern::{Kern, KernErr};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};
use crate::vnix::core::unit::{Unit, Int, UnitNew, UnitAs, UnitParse, UnitModify, UnitReadAsyncI, UnitTypeReadAsync};


pub const SERV_PATH: &'static str = "math.calc";
const SERV_HELP: &'static str = "{
    name:math.calc
    info:`Service for integer mathematical computation`
    tut:[
        {
            info:`Negate sign of number`
            com:(neg 5)@math.calc
            res:-5
        }
        {
            info:`Absolute value of number`
            com:(abs -3)@math.calc
            res:3
        }
        {
            info:`Increment number`
            com:(inc 2)@math.calc
            res:3
        }
        {
            info:`Decrement number`
            com:(dec 3)@math.calc
            res:2
        }
        {
            info:`Square number`
            com:(sqr 2)@math.calc
            res:4
        }
        {
            info:`Integer part of number square root`
            com:(sqrt 10)@math.calc
            res:3
        }
        {
            info:`Compute number factorial`
            com:(fac 20)@math.calc
            res:2432902008176640000
        }
        {
            info:`Compute sum of two numbers`
            com:(sum (1 2))@math.calc
            res:3
        }
        {
            info:`Compute sum of numbers list`
            com:(sum [1 2 3])@math.calc
            res:6
        }
        {
            info:`Compute subtract of two numbers`
            com:(sub (3 4))@math.calc
            res:1
        }
        {
            info:`Compute power of two numbers`
            com:(pow (21 12))@math.calc
            res:7355827511386641
        }
        {
            info:`Compute multiplication of two numbers`
            com:(mul (3 4))@math.calc
            res:12
        }
        {
            info:`Compute division of two numbers`
            com:(div (12 4))@math.calc
            res:3
        }
        {
            info:`Compute euclidean division of two numbers`
            com:(mod (5 2))@math.calc
            res:1
        }
        {
            info:`Find minimal number from presented`
            com:(min (1 2))@math.calc
            res:1
        }
        {
            info:`Find maximum number from presented`
            com:(max (1 2))@math.calc
            res:2
        }
    ]
    man:{
        neg:{
            info:`Negate sign of number`
            schm:[
                (neg int)
                {neg:int}
            ]
            tut:@tut.0
        }
        abs:{
            info:`Absolute value of number`
            schm:[
                (abs int)
                {abs:int}
            ]
            tut:@tut.1
        }
        inc:{
            info:`Increment number`
            schm:[
                (inc int)
                {inc:int}
            ]
            tut:@tut.2
        }
        dec:{
            info:`Decrement number`
            schm:[
                (dec int)
                {dec:int}
            ]
            tut:@tut.3
        }
        sqr:{
            info:`Square number`
            schm:[
                (sqr int)
                {sqr:int}
            ]
            tut:@tut.4
        }
        sqrt:{
            info:`Integer part of number square root`
            schm:[
                (sqrt int)
                {sqrt:int}
            ]
            tut:@tut.5
        }
        fac:{
            info:`Compute number factorial`
            schm:[
                (fac int)
                {fac:int}
            ]
            tut:@tut.6
        }
        sum:{
            info:`Compute sum of numbers`
            schm:[
                (sum (a b))
                (sum [a b c])
                {sum:(a b)}
                {sum:[a b c]}
            ]
            tut:[@tut.7 @tut.8]
        }
        sub:{
            info:`Compute subtract of numbers`
            schm:[
                (sub (a b))
                (sub [a b c])
                {sub:(a b)}
                {sub:[a b c]}
            ]
            tut:@tut.9
        }
        pow:{
            info:`Compute power of numbers`
            schm:[
                (pow (a b))
                (pow [a b c])
                {pow:(a b)}
                {pow:[a b c]}
            ]
            tut:@tut.10
        }
        mul:{
            info:`Compute multiplication of numbers`
            schm:[
                (mul (a b))
                (mul [a b c])
                {mul:(a b)}
                {mul:[a b c]}
            ]
            tut:@tut.11
        }
        div:{
            info:`Compute integer part of numbers division`
            schm:[
                (div (a b))
                (div [a b c])
                {div:(a b)}
                {div:[a b c]}
            ]
            tut:@tut.12
        }
        mod:{
            info:`Compute numbers Euclidean division`
            schm:[
                (mod (a b))
                (mod [a b c])
                {mod:(a b)}
                {mod:[a b c]}
            ]
            tut:@tut.13
        }
        min:{
            info:`Find minimal number from presented`
            schm:[
                (min (a b))
                (min [a b c])
                {min:(a b)}
                {min:[a b c]}
            ]
            tut:@tut.14
        }
        max:{
            info:`Find maximum number from presented`
            schm:[
                (max (a b))
                (max [a b c])
                {max:(a b)}
                {max:[a b c]}
            ]
            tut:@tut.15
        }
    }
}";

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
            "min" => a.0.as_ref().clone().min(b.0.as_ref().clone()),
            "max" => a.0.as_ref().clone().max(b.0.as_ref().clone()),
            _ => return None
        };
        Some(Int(Rc::new(res)))
    }).flatten()
}

fn single_op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Int> {
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

fn multi_args_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Vec<Int>> {
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

fn multi_op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Int> {
    thread!({
        // (op (v0 v1)) | (op [v ..])
        if let Some(((op, args), ath)) = as_async!(msg, as_pair, ath, orig, kern)? {
            let (op, ath) = maybe!(as_async!(op, as_str, ath, orig, kern));
            let (args, ath) = maybe!(thread_await!(multi_args_int(ath.clone(), orig.clone(), args, kern)));

            return Ok(calc_multi_op_int(&op, args).map(|v| (v, ath)))
        }

        let ops = ["sum", "sub", "pow", "mul", "div", "mod", "min", "max"];
        for op in ops {
            if let Some((args, ath)) = as_map_find_async!(msg, op, ath, orig, kern)? {
                let (args, ath) = maybe!(thread_await!(multi_args_int(ath.clone(), orig.clone(), args, kern)));
                return Ok(calc_multi_op_int(&op, args).map(|v| (v, ath)))
            }
        }
        Ok(None)
    })
}

fn op_int(ath: Rc<String>, orig: Unit, msg: Unit, kern: &Mutex<Kern>) -> UnitTypeReadAsync<Int> {
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

pub fn help_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let s = maybe_ok!(msg.msg.clone().as_str());
        let help = Unit::parse(SERV_HELP.chars()).map_err(|e| KernErr::ParseErr(e))?.0;
        yield;

        let res = match s.as_str() {
            "help" => help,
            "help.name" => maybe_ok!(help.find(["name"].into_iter())),
            "help.info" => maybe_ok!(help.find(["info"].into_iter())),
            "help.tut" => maybe_ok!(help.find(["tut"].into_iter())),
            "help.man" => maybe_ok!(help.find(["man"].into_iter())),
            _ => return Ok(None)
        };

        let _msg = Unit::map(&[
            (Unit::str("msg"), res)
        ]);
        kern.lock().msg(&msg.ath, _msg).map(|msg| Some(msg))
    })
}

pub fn calc_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        let ath = Rc::new(msg.ath.clone());

        if let Some((val, ath)) = thread_await!(op_int(ath.clone(), msg.msg.clone(), msg.msg.clone(), kern))? {
            let msg = Unit::map(&[
                (Unit::str("msg"), Unit::int_share(val.0))]
            );
            return kern.lock().msg(&ath, msg).map(|msg| Some(msg))
        }

        Ok(Some(msg))
    })
}