use core::ops::Deref;
use core::str::Chars;
use core::fmt::{Display, Formatter};

use alloc::format;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};

use super::kern::{Kern, KernErr};


#[derive(Debug)]
pub enum UnitParseErr {
    NotNone,
    NotBool,
    NotByte,
    NotInt,
    NotDec,
    NotStr,
    NotRef,
    NotPair,
    NotList,
    NotMap,
    NotUnit,
    NotClosedBrackets,
    NotClosedQuotes,
    MissedSeparator,
    UnexpectedEnd,
    MissedDot,
    MissedPartAfterDot,
    RefNotString,
    RefInvalidPath
}

#[derive(Debug, PartialEq, Clone)]
pub enum Unit {
    None,
    Bool(bool),
    Byte(u8),
    Int(i32),
    Dec(f32),
    Str(String),
    Ref(Vec<String>),
    Pair((Box<Unit>, Box<Unit>)),
    Lst(Vec<Unit>),
    Map(Vec<(Unit, Unit)>)
}

impl Eq for Unit {}

impl Display for Unit {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::None => write!(f, "-"),
            Unit::Bool(b) => {
                if *b {
                    write!(f, "t")
                } else {
                    write!(f, "f")
                }
            },
            Unit::Byte(b) => write!(f, "{:#02x}", b),
            Unit::Int(i) => write!(f, "{}", i),
            Unit::Dec(d) => write!(f, "{}", d),
            Unit::Str(s) => {
                if s.as_str().chars().all(|c| c.is_alphanumeric() || c == '.') {
                    write!(f, "{}", s)
                } else {
                    write!(f, "`{}`", s)
                }
            },
            Unit::Ref(path) => write!(f, "@{}", path.join(".")),
            Unit::Pair(p) => write!(f, "({} {})", p.0, p.1),
            Unit::Lst(lst) => {
                write!(f, "[")?;

                for (i, u) in lst.iter().enumerate() {
                    if i == lst.len() - 1 {
                        write!(f, "{}", u)?;
                    } else {
                        write!(f, "{} ", u)?;
                    }
                }

                write!(f, "]")
            },
            Unit::Map(map) => {
                write!(f, "{{")?;

                for (i, (u0, u1)) in map.iter().enumerate() {
                    if i == map.len() - 1 {
                        write!(f, "{}:{}", u0, u1)?;
                    } else {
                        write!(f, "{}:{} ", u0, u1)?;
                    }
                }

                write!(f, "}}")
            }
        }
    }
}

impl Unit {
    fn parse_ch<'a>(ch: char, it: Chars<'a>) -> (bool, Chars<'a>) {
        let mut tmp = it.clone();

        if let Some(c) = tmp.next() {
            if c == ch {
                return (true, tmp)
            }
        }
        (false, it)
    }

    fn parse_ws<'a>(it: Chars<'a>) -> (bool, Chars<'a>) {
        let mut tmp = it.clone();

        if let Some(c) = tmp.next() {
            let mut it = tmp;

            if c.is_ascii_whitespace() {
                tmp = it.clone();

                while let Some(c) = it.next() {
                    if !c.is_ascii_whitespace() {
                        break;
                    }
                    tmp = it.clone();
                }

                return (true, tmp);
            }
        }

        (false, it)
    }

    fn parse_none<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        let (ok, tmp) = Unit::parse_ch('-', it);

        if ok {
            return Ok((Unit::None, tmp));
        }

        Err(KernErr::ParseErr(UnitParseErr::NotNone))
    }

    fn parse_bool<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        let (ok_t, tmp_t) = Unit::parse_ch('t', it.clone());
        let (ok_f, tmp_f) = Unit::parse_ch('f', it);

        let mut tmp = if ok_t {tmp_t.clone()} else {tmp_f.clone()};

        if let Some(c) = tmp.next() {
            if c.is_alphanumeric() {
                return Err(KernErr::ParseErr(UnitParseErr::NotBool));
            }
        }

        if ok_t {
            return Ok((Unit::Bool(true), tmp_t))
        }

        if ok_f {
            return Ok((Unit::Bool(false), tmp_f))
        }

        Err(KernErr::ParseErr(UnitParseErr::NotBool))
    }

    fn parse_byte<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(s) = it.as_str().get(0..4) {
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();

            if let Ok(v) = u8::from_str_radix(s.trim_start_matches("0x"), 16) {
                return Ok((Unit::Byte(v), it))
            }
        }

        Err(KernErr::ParseErr(UnitParseErr::NotByte))
    }

    fn parse_int<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        let mut s = String::new();
        let mut tmp = it.clone();

        while let Some(c) = it.next() {
            if !c.is_numeric() {
                break;
            }

            s.push(c);
            tmp = it.clone();
        }

        if let Ok(v) = s.parse::<i32>() {
            return Ok((Unit::Int(v), tmp));
        }

        Err(KernErr::ParseErr(UnitParseErr::NotInt))
    }

    fn parse_dec<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Ok((fst, mut it)) = Unit::parse_int(it) {
            let (ok, tmp) = Unit::parse_ch('.', it);

            if !ok {
                return Err(KernErr::ParseErr(UnitParseErr::MissedDot));
            }

            it = tmp;

            if let Ok((scd, it)) = Unit::parse_int(it) {
                let s = format!("{}.{}", fst, scd);
                let out = s.parse::<f32>().map_err(|_| KernErr::ParseErr(UnitParseErr::NotDec))?;

                return Ok((Unit::Dec(out), it));
            } else {
                return Err(KernErr::ParseErr(UnitParseErr::MissedPartAfterDot));
            }
        }
        Err(KernErr::ParseErr(UnitParseErr::NotDec))
    }

    fn parse_str<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), KernErr> {
        if let Some(c) = it.next() {
            // `complex string`
            if c == '`' {
                let mut s = String::new();
                let mut tmp = it.clone();

                while let Some(c) = it.next() {
                    if c == '`' {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                if let Some(c) = tmp.next() {
                    if c == '`' {
                        return Ok((Unit::Str(s), tmp));
                    } else {
                        return Err(KernErr::ParseErr(UnitParseErr::NotClosedQuotes));
                    }
                } else {
                    return Err(KernErr::ParseErr(UnitParseErr::NotClosedQuotes));
                }
            }

            // abc.123
            if c.is_alphanumeric() || c == '.'{
                let mut s = String::new();
                let mut tmp = it.clone();

                s.push(c);

                while let Some(c) = it.next() {
                    if !(c.is_alphanumeric() || c == '.') {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                return Ok((Unit::Str(s), tmp));
            }
        }
        Err(KernErr::ParseErr(UnitParseErr::NotStr))
    }

    fn parse_ref<'a>(mut it: Chars<'a>, _kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        let (ok, tmp) = Unit::parse_ch('@', it);

        if !ok {
            return Err(KernErr::ParseErr(UnitParseErr::NotRef));
        }

        it = tmp;

        let tmp = Unit::parse_str(it)?;

        if let Unit::Str(path) = tmp.0 {
            let path = path.split(".").map(|s| s.to_string()).collect::<Vec<_>>();

            for p in &path {
                if !p.chars().all(|c| c.is_alphanumeric()) {
                    return Err(KernErr::ParseErr(UnitParseErr::RefInvalidPath));
                }
            }

            return Ok((Unit::Ref(path), tmp.1));
        }
        return Err(KernErr::ParseErr(UnitParseErr::RefNotString));
    }

    fn parse_pair<'a>(mut it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        let (ok, tmp) = Unit::parse_ch('(', it);

        if !ok {
            return Err(KernErr::ParseErr(UnitParseErr::NotPair))
        }

        it = tmp;

        let u0 = Unit::parse(it, kern)?;
        it = u0.1;

        let (ok, tmp) = Unit::parse_ws(it);

        if !ok {
            return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
        }

        it = tmp;

        let u1 = Unit::parse(it, kern)?;
        it = u1.1;

        let (ok, tmp) = Unit::parse_ch(')', it);

        if !ok {
            return Err(KernErr::ParseErr(UnitParseErr::NotClosedBrackets));
        }

        it = tmp;

        return Ok((
            Unit::Pair((
                Box::new(u0.0),
                Box::new(u1.0)
            )),
            it
        ));
    }

    fn parse_list<'a>(mut it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        let (ok, tmp) = Unit::parse_ch('[', it);

        if !ok {
            return Err(KernErr::ParseErr(UnitParseErr::NotList));
        }

        it = tmp;

        let mut lst = Vec::new();

        loop {
            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let u = Unit::parse(it, kern)?;
            lst.push(u.0);
            it = u.1;

            let (ok_ws, tmp) = Unit::parse_ws(it);
            it = tmp;

            let (ok, tmp) = Unit::parse_ch(']', it);

            if ok {
                it = tmp;
                return Ok((Unit::Lst(lst), it))
            } else if !ok_ws {
                return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
            }

            it = tmp;
        }
    }

    fn parse_map<'a>(mut it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        let (ok, tmp) = Unit::parse_ch('{', it);

        if !ok {
            return Err(KernErr::ParseErr(UnitParseErr::NotMap));
        }

        it = tmp;

        let mut map = Vec::new();

        loop {
            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let u0 = Unit::parse(it, kern)?;
            it = u0.1;

            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let (ok, tmp) = Unit::parse_ch(':', it);

            if !ok {
                return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
            }

            it = tmp;

            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let u1 = Unit::parse(it, kern)?;
            it = u1.1;

            map.push((u0.0, u1.0));

            let (ok_ws, tmp) = Unit::parse_ws(it);
            it = tmp;

            let (ok, tmp) = Unit::parse_ch('}', it);

            if ok {
                it = tmp;
                return Ok((Unit::Map(map), it))
            } else if !ok_ws {
                return Err(KernErr::ParseErr(UnitParseErr::MissedSeparator));
            }

            it = tmp;
        }
    }

    pub fn parse<'a>(it: Chars<'a>, kern: &mut Kern) -> Result<(Self, Chars<'a>), KernErr> {
        // none
        if let Ok((u, it)) = Unit::parse_none(it.clone()) {
            return Ok((u, it));
        }

        // bool
        if let Ok((u, it)) = Unit::parse_bool(it.clone()) {
            return Ok((u, it));
        }

        // byte
        if let Ok((u, it)) = Unit::parse_byte(it.clone()) {
            return Ok((u, it));
        }

        // dec
        if let Ok((u, it)) = Unit::parse_dec(it.clone()) {
            return Ok((u, it));
        }

        // int
        if let Ok((u, it)) = Unit::parse_int(it.clone()) {
            return Ok((u, it));
        }

        // str
        if let Ok((u, it)) = Unit::parse_str(it.clone()) {
            return Ok((u, it));
        }

        // pair
        if let Ok((u, it)) = Unit::parse_pair(it.clone(), kern) {
            return Ok((u, it));
        }

        if let Ok((u, it)) = Unit::parse_ref(it.clone(), kern) {
            return Ok((u, it));
        }

        // list
        if let Ok((u, it)) = Unit::parse_list(it.clone(), kern) {
            return Ok((u, it));
        }

        // map
        if let Ok((u, it)) = Unit::parse_map(it.clone(), kern) {
            return Ok((u, it));
        }

        Err(KernErr::ParseErr(UnitParseErr::NotUnit))
    }

    fn find_unit_loc<'a, I>(&self, glob: &Unit, path: &mut I) -> Option<Unit> where I: Iterator<Item = &'a String> {
        if let Some(curr) = path.next() {
            if let Unit::Ref(n_path) = self {
                return glob.find_unit(&mut n_path.iter());
            }

            if let Unit::Pair(p) = self {
                if curr == "0" {
                    return p.0.deref().find_unit_loc(glob, path);
                } else if curr == "1" {
                    return p.1.deref().find_unit_loc(glob, path);
                }
            }

            if let Unit::Lst(lst) = self {
                let idx = curr.parse::<usize>().ok()?;
                if let Some(u) = lst.get(idx) {
                    return u.find_unit_loc(glob, path);
                }
            }

            if let Unit::Map(m) = self {
                return m.iter().filter_map(|(u0, u1)| Some((u0.as_str()?, u1)))
                        .find(|(s, _)| *s == *curr)
                        .map(|(_, u)| u.find_unit_loc(glob, path)).flatten();
            }

            return None;
        } else {
            if let Unit::Ref(n_path) = self {
                return glob.find_unit(&mut n_path.iter());
            }

            return Some(self.clone());
        }
    }

    pub fn find_unit<'a, I>(&self, path: &mut I) -> Option<Unit> where I: Iterator<Item = &'a String> {
        self.find_unit_loc(self, path)
    }

    pub fn find_bool<'a, I>(&self, path: &mut I) -> Option<bool> where I: Iterator<Item = &'a String> {
        self.find_unit(path).map(|u| u.as_bool()).flatten()
    }

    pub fn find_int<'a, I>(&self, path: &mut I) -> Option<i32> where I: Iterator<Item = &'a String> {
        self.find_unit(path).map(|u| u.as_int()).flatten()
    }

    pub fn find_str<'a, I>(&self, path: &mut I) -> Option<String> where I: Iterator<Item = &'a String> {
        self.find_unit(path).map(|u| u.as_str()).flatten()
    }

    pub fn find_vec<'a, I>(&self, path: &mut I) -> Option<Vec<Unit>> where I: Iterator<Item = &'a String> {
        self.find_unit(path).map(|u| u.as_vec()).flatten()
    }

    pub fn as_none(&self) -> Option<()> {
        if let Unit::None = self {
            return Some(())
        }
        None
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Unit::Bool(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_byte(&self) -> Option<u8> {
        if let Unit::Byte(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_int(&self) -> Option<i32> {
        if let Unit::Int(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_dec(&self) -> Option<f32> {
        if let Unit::Dec(v) = self {
            return Some(*v)
        }
        None
    }

    pub fn as_str(&self) -> Option<String> {
        if let Unit::Str(s) = self {
            return Some(s.clone())
        }
        None
    }

    pub fn as_ref(&self) -> Option<Vec<String>> {
        if let Unit::Ref(path) = self {
            return Some(path.clone());
        }
        None
    }

    pub fn as_pair(&self) -> Option<(Box<Unit>, Box<Unit>)> {
        if let Unit::Pair((u0, u1)) = self {
            return Some((u0.clone(), u1.clone()))
        }
        None
    }

    pub fn as_vec(&self) -> Option<Vec<Unit>> {
        if let Unit::Lst(lst) = self {
            return Some(lst.clone());
        }
        None
    }

    pub fn as_map(&self) -> Option<Vec<(Unit, Unit)>> {
        if let Unit::Map(m) = self {
            return Some(m.clone());
        }
        None
    }
}
