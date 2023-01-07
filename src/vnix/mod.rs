use heapless::{LinearMap, Vec};

pub mod unit;
pub mod msg;
pub mod user;
pub mod serv;
pub mod kern;

use unit::Unit;
use msg::Msg;
use user::Usr;
use kern::Kern;

use self::kern::KernErr;

pub fn vnix_entry(mut kern: Kern) -> Result<(), KernErr> {
    kern.cli.reset().map_err(|e| KernErr::CLIErr(e))?;

    // prepare user
    let _super = Usr {
        name: "super".into()
    };

    // prepare message
    let u0 = kern.unit(Unit::Str("msg".into()))?;
    let u1 = kern.unit(Unit::Str("Hello, vnix ®!".into()))?;

    let u2 = kern.unit(Unit::Str("a".into()))?;
    let u3 = kern.unit(Unit::None)?;

    let u4 = kern.unit(Unit::Int(1))?;
    let u5 = kern.unit(Unit::Dec(3.14))?;

    let u8 = kern.unit(Unit::Str("c".into()))?;
    let u9 = kern.unit(Unit::Dec(2.74))?;
    let u10 = kern.unit(Unit::Pair((u8, u9)))?;

    let mut u6_lst = Vec::new();
    u6_lst.push(kern.dup(&u2)?).map_err(|_| KernErr::MemoryOut)?;
    u6_lst.push(u4).map_err(|_| KernErr::MemoryOut)?;
    u6_lst.push(u5).map_err(|_| KernErr::MemoryOut)?;
    u6_lst.push(kern.dup(&u10)?).map_err(|_| KernErr::MemoryOut)?;

    let u6 = kern.unit(Unit::Lst(u6_lst))?;
    let u7 = kern.unit(Unit::Str("b".into()))?;

    let mut u_map = LinearMap::new();
    u_map.insert(u0, u1).map_err(|_| KernErr::MemoryOut)?;
    u_map.insert(u2, u3).map_err(|_| KernErr::MemoryOut)?;
    u_map.insert(u7, u6).map_err(|_| KernErr::MemoryOut)?;

    let msg = Msg {
        ath: _super,
        msg: kern.unit(Unit::Map(u_map))?
    };

    // run
    kern.cli.println(core::format_args!("{}", msg));

    loop {

    }
}
