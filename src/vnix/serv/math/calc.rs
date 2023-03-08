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
use crate::vnix::core::unit::{Unit, SchemaPair, SchemaUnit, Schema};
use crate::vnix::core::serv::{ServHlrAsync, ServInfo};


pub const SERV_PATH: &'static str = "math.calc";
pub const SERV_HELP: &'static str = "Service for integer mathematical computation\nExample: {sum:[1 2 3]}@math.calc";


pub fn calc_hlr(msg: Msg, _serv: ServInfo, kern: &Mutex<Kern>) -> ServHlrAsync {
    thread!({
        yield;
        Ok(Some(msg))
    })
}