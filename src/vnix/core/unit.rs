use core::str::Chars;
use core::fmt::{Display, Formatter};

use alloc::{format, vec};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};


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
    Pair(Box<Unit>, Box<Unit>),
    Lst(Vec<Unit>),
    Map(Vec<(Unit, Unit)>)
}

pub trait Schema: Clone {
    type Out;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out>;

    fn find_deep(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        match u {
            Unit::Ref(path) => {
                if let Some(u) = Unit::find_ref(path.into_iter().cloned(), glob) {
                    return self.find_deep(glob, &u);
                }
            },
            _ => return self.find(glob, u)
        }
        None
    }

    fn find_loc(&self, u: &Unit) -> Option<Self::Out> {
        self.find_deep(u, u)
    }
}

#[derive(Debug, Clone)]
pub struct SchemaNone;

#[derive(Debug, Clone)]
pub struct SchemaBool;

#[derive(Debug, Clone)]
pub struct SchemaByte;

#[derive(Debug, Clone)]
pub struct SchemaInt;

#[derive(Debug, Clone)]
pub struct SchemaDec;

#[derive(Debug, Clone)]
pub struct SchemaStr;

#[derive(Debug, Clone)]
pub struct SchemaRef;

#[derive(Debug, Clone)]
pub struct  SchemaUnit;

#[derive(Debug, Clone)]
pub struct SchemaPair<A, B>(pub A, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaSeq<A>(pub A) where A: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapSeq<A, B>(pub A, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapEntry<A>(pub Unit, pub A) where A: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMap<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapFirstRequire<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapSecondRequire<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub struct SchemaMapRequire<A, B>(pub SchemaMapEntry<A>, pub B) where A: Schema, B: Schema;

#[derive(Debug, Clone)]
pub enum Or<A, B> {
    First(A),
    Second(B)
}

#[derive(Debug, Clone)]
pub struct SchemaOr<A, B>(pub A, pub B) where A: Schema, B: Schema;

pub trait FromUnit: Sized {
    fn from_unit_loc(u: &Unit) -> Option<Self>;

    fn from_unit(glob: &Unit, u: &Unit) -> Option<Self> {
        Self::from_unit_loc(u)
    }
}

pub struct DisplayShort<'a>(pub &'a Unit, pub usize);

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
                if s.as_str().chars().all(|c| c.is_alphanumeric() || c == '.' || c == '#' || c == '_') {
                    write!(f, "{}", s)
                } else {
                    write!(f, "`{}`", s)
                }
            },
            Unit::Ref(path) => write!(f, "@{}", path.join(".")),
            Unit::Pair(u0, u1) => write!(f, "({} {})", u0, u1),
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

impl<'a> Display for DisplayShort<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.0 {
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
                let mut s = s.clone();
                s.truncate(self.1);

                if s.len() >= self.1 {
                    s = format!("{}..", s);
                }

                if s.as_str().chars().all(|c| c.is_alphanumeric() || c == '.' || c == '#' || c == '_') {
                    write!(f, "{}", s)
                } else {
                    write!(f, "`{}`", s)
                }
            },
            Unit::Ref(path) => write!(f, "@{}", path.join(".")),
            Unit::Pair(u0, u1) => write!(f, "({} {})", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1)),
            Unit::Lst(lst) => {
                write!(f, "[")?;

                for (i, u) in lst.iter().take(self.1).enumerate() {
                    if i == lst.len().min(self.1) - 1 && lst.len() > self.1 {
                        write!(f, "{}..", DisplayShort(&u, self.1))?;
                    } else if i == lst.len().min(self.1) - 1 {
                        write!(f, "{}", DisplayShort(&u, self.1))?;
                    } else {
                        write!(f, "{} ", DisplayShort(&u, self.1))?;
                    }
                }

                write!(f, "]")
            },
            Unit::Map(map) => {
                write!(f, "{{")?;

                for (i, (u0, u1)) in map.iter().take(self.1).enumerate() {
                    if i == map.len().min(self.1) - 1 && map.len() > self.1 {
                        write!(f, "{}:{}..", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1))?;
                    } else if  i == map.len().min(self.1) - 1 {
                        write!(f, "{}:{}", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1))?;
                    } else {
                        write!(f, "{}:{} ", DisplayShort(&u0, self.1), DisplayShort(&u1, self.1))?;
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

    fn parse_none<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (ok, tmp) = Unit::parse_ch('-', it);

        if ok {
            return Ok((Unit::None, tmp));
        }

       Err(UnitParseErr::NotNone)
    }

    fn parse_bool<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (ok_t, tmp_t) = Unit::parse_ch('t', it.clone());
        let (ok_f, tmp_f) = Unit::parse_ch('f', it);

        let mut tmp = if ok_t {tmp_t.clone()} else {tmp_f.clone()};

        if let Some(c) = tmp.next() {
            if c.is_alphanumeric() {
                return Err(UnitParseErr::NotBool);
            }
        }

        if ok_t {
            return Ok((Unit::Bool(true), tmp_t))
        }

        if ok_f {
            return Ok((Unit::Bool(false), tmp_f))
        }

        Err(UnitParseErr::NotBool)
    }

    fn parse_byte<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        if let Some(s) = it.as_str().get(0..4) {
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();
            it.next().unwrap();

            if let Ok(v) = u8::from_str_radix(s.trim_start_matches("0x"), 16) {
                return Ok((Unit::Byte(v), it))
            }
        }

        Err(UnitParseErr::NotByte)
    }

    fn parse_int<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let mut s = String::new();
        let mut tmp = it.clone();

        while let Some(c) = it.next() {
            if !(c.is_numeric() || c == '-') {
                break;
            }

            s.push(c);
            tmp = it.clone();
        }

        if let Ok(v) = s.parse::<i32>() {
            return Ok((Unit::Int(v), tmp));
        }

        Err(UnitParseErr::NotInt)
    }

    fn parse_dec<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        if let Ok((fst, mut it)) = Unit::parse_int(it) {
            let (ok, tmp) = Unit::parse_ch('.', it);

            if !ok {
                return Err(UnitParseErr::MissedDot);
            }

            it = tmp;

            if let Ok((scd, it)) = Unit::parse_int(it) {
                let s = format!("{}.{}", fst, scd);
                let out = s.parse::<f32>().map_err(|_| UnitParseErr::NotDec)?;

                return Ok((Unit::Dec(out), it));
            } else {
                return Err(UnitParseErr::MissedPartAfterDot);
            }
        }
        Err(UnitParseErr::NotDec)
    }

    fn parse_str<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
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
                        return Err(UnitParseErr::NotClosedQuotes);
                    }
                } else {
                    return Err(UnitParseErr::NotClosedQuotes);
                }
            }

            // 'complex string'
            if c == '\'' {
                let mut s = String::new();
                let mut tmp = it.clone();

                while let Some(c) = it.next() {
                    if c == '\'' {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                if let Some(c) = tmp.next() {
                    if c == '\'' {
                        return Ok((Unit::Str(s), tmp));
                    } else {
                        return Err(UnitParseErr::NotClosedQuotes);
                    }
                } else {
                    return Err(UnitParseErr::NotClosedQuotes);
                }
            }

            // "complex string"
            if c == '"' {
                let mut s = String::new();
                let mut tmp = it.clone();

                while let Some(c) = it.next() {
                    if c == '"' {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                if let Some(c) = tmp.next() {
                    if c == '"' {
                        return Ok((Unit::Str(s), tmp));
                    } else {
                        return Err(UnitParseErr::NotClosedQuotes);
                    }
                } else {
                    return Err(UnitParseErr::NotClosedQuotes);
                }
            }

            // abc.123#
            if c.is_alphanumeric() || c == '.' || c == '#' || c == '_' {
                let mut s = String::new();
                let mut tmp = it.clone();

                s.push(c);

                while let Some(c) = it.next() {
                    if !(c.is_alphanumeric() || c == '.' || c == '#' || c == '_') {
                        break;
                    }

                    s.push(c);
                    tmp = it.clone();
                }

                return Ok((Unit::Str(s), tmp));
            }
        }
        Err(UnitParseErr::NotStr)
    }

    fn parse_ref<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (ok, tmp) = Unit::parse_ch('@', it);

        if !ok {
            return Err(UnitParseErr::NotRef);
        }

        it = tmp;

        let tmp = Unit::parse_str(it)?;

        if let Unit::Str(path) = tmp.0 {
            let path = path.split(".").map(|s| s.to_string()).collect::<Vec<_>>();

            for p in &path {
                if !p.chars().all(|c| c.is_alphanumeric() || c == '#' || c == '_') {
                    return Err(UnitParseErr::RefInvalidPath);
                }
            }

            return Ok((Unit::Ref(path), tmp.1));
        }
        return Err(UnitParseErr::RefNotString);
    }

    fn parse_pair<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (ok, tmp) = Unit::parse_ch('(', it);

        if !ok {
            return Err(UnitParseErr::NotPair)
        }

        it = tmp;

        let u0 = Unit::parse(it)?;
        it = u0.1;

        let (ok, tmp) = Unit::parse_ws(it);

        if !ok {
            return Err(UnitParseErr::MissedSeparator);
        }

        it = tmp;

        let u1 = Unit::parse(it)?;
        it = u1.1;

        let (ok, tmp) = Unit::parse_ch(')', it);

        if !ok {
            return Err(UnitParseErr::NotClosedBrackets);
        }

        it = tmp;

        return Ok((
            Unit::Pair(
                Box::new(u0.0),
                Box::new(u1.0)
            ),
            it
        ));
    }

    fn parse_list<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (ok, tmp) = Unit::parse_ch('[', it);

        if !ok {
            return Err(UnitParseErr::NotList);
        }

        it = tmp;

        let mut lst = Vec::new();

        loop {
            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let u = Unit::parse(it)?;
            lst.push(u.0);
            it = u.1;

            let (ok_ws, tmp) = Unit::parse_ws(it);
            it = tmp;

            let (ok, tmp) = Unit::parse_ch(']', it);

            if ok {
                it = tmp;
                return Ok((Unit::Lst(lst), it))
            } else if !ok_ws {
                return Err(UnitParseErr::MissedSeparator);
            }

            it = tmp;
        }
    }

    fn parse_map<'a>(mut it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
        let (ok, tmp) = Unit::parse_ch('{', it);

        if !ok {
            return Err(UnitParseErr::NotMap);
        }

        it = tmp;

        let mut map = Vec::new();

        loop {
            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let u0 = Unit::parse(it)?;
            it = u0.1;

            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let (ok, tmp) = Unit::parse_ch(':', it);

            if !ok {
                return Err(UnitParseErr::MissedSeparator);
            }

            it = tmp;

            let (_, tmp) = Unit::parse_ws(it);
            it = tmp;

            let u1 = Unit::parse(it)?;
            it = u1.1;

            map.push((u0.0, u1.0));

            let (ok_ws, tmp) = Unit::parse_ws(it);
            it = tmp;

            let (ok, tmp) = Unit::parse_ch('}', it);

            if ok {
                it = tmp;
                return Ok((Unit::Map(map), it))
            } else if !ok_ws {
                return Err(UnitParseErr::MissedSeparator);
            }

            it = tmp;
        }
    }

    pub fn parse<'a>(it: Chars<'a>) -> Result<(Self, Chars<'a>), UnitParseErr> {
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

        // none
        if let Ok((u, it)) = Unit::parse_none(it.clone()) {
            return Ok((u, it));
        }

        // str
        if let Ok((u, it)) = Unit::parse_str(it.clone()) {
            return Ok((u, it));
        }

        // pair
        if let Ok((u, it)) = Unit::parse_pair(it.clone()) {
            return Ok((u, it));
        }

        if let Ok((u, it)) = Unit::parse_ref(it.clone()) {
            return Ok((u, it));
        }

        // list
        if let Ok((u, it)) = Unit::parse_list(it.clone()) {
            return Ok((u, it));
        }

        // map
        if let Ok((u, it)) = Unit::parse_map(it.clone()) {
            return Ok((u, it));
        }

        Err(UnitParseErr::NotUnit)
    }

    pub fn find_ref<I>(mut path: I, u: &Unit) -> Option<Unit> where I: Iterator<Item = String> {
        match u {
            Unit::Map(m) => {
                if let Some(path_s) = path.next() {
                    if let Some((_, next)) = m.iter().filter_map(|(u0, u1)| Some((u0.as_str()?, u1))).find(|(s, _)| *s == path_s) {
                        return Unit::find_ref(path, next);
                    } else if path_s == "all" {
                        return Some(u.clone());
                    }
                } else {
                    return Some(u.clone());
                }
            },
            Unit::Lst(lst) => {
                if let Some(path_s) = path.next() {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        if let Some(next) = lst.get(idx) {
                            return Unit::find_ref(path, next);
                        }
                    }
                } else {
                    return Some(u.clone())
                }
            },
            Unit::Pair(u0, u1) => {
                if let Some(path_s) = path.next() {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        match idx {
                            0 => return Unit::find_ref(path, u0),
                            1 => return Unit::find_ref(path, u1),
                            _ => ()
                        }
                    }
                } else {
                    return Some(u.clone())
                }
            }
            _ => return Some(u.clone())
        }
        None
    }

    pub fn merge_ref<I>(mut path: I, val: Unit, u: Unit) -> Option<Unit> where I: Iterator<Item = String> {
        if let Some(path_s) = path.next() {
            match u {
                Unit::Map(m) => {
                    if let Some((_, next)) = m.iter().filter_map(|(u0, u1)| Some((u0.as_str()?, u1))).find(|(s, _)| *s == path_s) {
                        let val = Unit::merge_ref(path, val, next.clone());
                        if let Some(val) = val {
                            let val = next.clone().merge(val);
                            let u = Unit::Map(vec![(Unit::Str(path_s), val)]);

                            return Some(Unit::Map(m).merge(u))
                        }
                    } else {
                        let val = Unit::merge_ref(path, val, Unit::Map(Vec::new()));
                        if let Some(val) = val {
                            let u = Unit::Map(vec![(Unit::Str(path_s), val)]);
                            return Some(Unit::Map(m).merge(u))
                        }
                    }
                },
                Unit::Lst(mut lst) => {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        if let Some(u) = lst.get_mut(idx) {
                            let val = Unit::merge_ref(path, val, u.clone());
                            if let Some(val) = val {
                               *u = val;
                            }
                        }
                    }
                    return Some(Unit::Lst(lst));
                },
                Unit::Pair(u0, u1) => {
                    if let Some(idx) = path_s.parse::<usize>().ok() {
                        match idx {
                            0 => {
                                let val = Unit::merge_ref(path, val, *u0);
                                if let Some(val) = val {
                                    return Some(Unit::Pair(Box::new(val), u1))
                                }
                            },
                            1 => {
                                let val = Unit::merge_ref(path, val, *u1);
                                if let Some(val) = val {
                                    return Some(Unit::Pair(u0, Box::new(val)))
                                }
                            },
                            _ => return Some(Unit::Pair(u0, u1))
                        };
                    }
                },
                _ => {
                    let val = Unit::merge_ref(path, val, Unit::Map(Vec::new()));

                    if let Some(val) = val {
                        let u = Unit::Map(vec![(Unit::Str(path_s), val)]);
                        return Some(u)
                    }
                }
            }
        } else {
            return Some(val);
        }
        None
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
        if let Unit::Pair(u0, u1) = self {
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

    pub fn as_vec_typed<A, B>(&self, f: B) -> Option<Vec<A>> where A: Clone, B: Fn(&Self) -> Option<A> {
        if let Unit::Lst(lst) = self {
            return Some(lst.iter().filter_map(|u| f(u)).collect());
        }
        None
    }

    pub fn as_map(&self) -> Option<Vec<(Unit, Unit)>> {
        if let Unit::Map(m) = self {
            return Some(m.clone());
        }
        None
    }

    pub fn as_map_find(&self, sch: &str) -> Option<Unit> {
        if let Unit::Map(m) = self {
            return m.iter()
                .filter_map(|(u0, u1)| Some((u0.as_str()?, u1)))
                .map(|(s, u)| {
                    if s == sch {
                        return Some(u.clone());
                    }
                    None
                }).next()?;
        }
        None
    }

    pub fn merge(self, u: Unit) -> Unit {
        match u {
            Unit::Map(m) => {
                if let Some(mut tmp) = self.as_map() {
                    tmp.retain(|(u, _)| {
                        m.iter().find(|(n, _)| u == n).is_none()
                    });

                    tmp.extend(m);
                    return Unit::Map(tmp);
                }
            },
            Unit::Pair(u0, u1) => {
                if self.as_pair().is_some() {
                    return Unit::Pair(u0, u1);
                }

                if let Some(mut tmp) = self.as_map() {
                    tmp.retain(|(u, _)| u.clone() == *u0);
                    tmp.push((*u0, *u1));
                    return Unit::Map(tmp);
                }

                if let Some(mut tmp) = self.as_vec() {
                    tmp.retain(|u| u.clone() == Unit::Pair(u0.clone(), u1.clone()));
                    tmp.push(Unit::Pair(u0.clone(), u1.clone()));
                    return Unit::Lst(tmp);
                }
            }
            Unit::Lst(lst) => {
                // if let Some(mut tmp) = self.as_vec() {
                //     tmp.retain(|u| {
                //         lst.iter().find(|n| *u == **n).is_none()
                //     });

                //     tmp.extend(lst);
                //     return Unit::Lst(tmp);
                // }
                if self.as_vec().is_some() {
                    return Unit::Lst(lst);
                }
            },
            _ => return u
        }
        self
    }
}

impl Schema for SchemaNone {
    type Out = ();

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::None = u {
            return Some(());
        }
        None
    }
}

impl Schema for SchemaBool {
    type Out = bool;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Bool(b) = u {
            return Some(*b);
        }
        None
    }
}

impl Schema for SchemaByte {
    type Out = u8;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Byte(b) = u {
            return Some(*b);
        }
        None
    }
}

impl Schema for SchemaInt {
    type Out = i32;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Int(v) = u {
            return Some(*v);
        }
        None
    }
}

impl Schema for SchemaDec {
    type Out = f32;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Dec(v) = u {
            return Some(*v);
        }
        None
    }
}

impl Schema for SchemaStr {
    type Out = String;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Str(s) = u {
            return Some(s.clone());
        }
        None
    }
}

impl Schema for SchemaUnit {
    type Out = Unit;

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        Some(u.clone())
    }
}

impl Schema for SchemaRef {
    type Out = Vec<String>;

    fn find_deep(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        self.find(glob, u)
    }

    fn find(&self, _glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Ref(path) = u {
            return Some(path.clone())
        }
        None
    }
}

impl<A, B> Schema for SchemaPair<A, B> where A: Schema, B: Schema {
    type Out = (A::Out, B::Out);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Pair(u0, u1) = u {
            return Some((self.0.find_deep(glob, u0)?, self.1.find_deep(glob, u1)?));
        }
        None
    }
}

impl<A> Schema for SchemaSeq<A> where A: Schema {
    type Out = Vec<A::Out>;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Lst(lst) = u {
            return Some(lst.iter().filter_map(|u| self.0.find_deep(glob, u)).collect());
        }
        None
    }
}

impl<A, B> Schema for SchemaMapSeq<A, B> where A: Schema, B: Schema {
    type Out = Vec<(A::Out, B::Out)>;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Map(m) = u {
            return Some(m.iter().filter_map(|(u0, u1)| Some((self.0.find_deep(glob, u0)?, self.1.find_deep(glob, u1)?))).collect());
        }
        None
    }
}

impl<A> Schema for SchemaMapEntry<A> where A: Schema {
    type Out = A::Out;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Unit::Map(m) = u {
            if let Some(u) = m.iter().find(|(u, _)| self.0 == u.clone()).map(|(_, u)| u) {
                return self.1.find_deep(glob, u);
            }
        }
        None
    }
}

impl<A, B> Schema for SchemaMap<A, B> where A: Schema, B: Schema {
    type Out = (Option<A::Out>, Option<B::Out>);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u), self.1.find_deep(glob, u)));
        }

        None
    }
}

impl<A, B> Schema for SchemaMapFirstRequire<A, B> where A: Schema, B: Schema {
    type Out = (A::Out, Option<B::Out>);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u)?, self.1.find_deep(glob, u)));
        }

        None
    }
}

impl<A, B> Schema for SchemaMapSecondRequire<A, B> where A: Schema, B: Schema {
    type Out = (Option<A::Out>, B::Out);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u), self.1.find_deep(glob, u)?));
        }

        None
    }
}

impl<A, B> Schema for SchemaMapRequire<A, B> where A: Schema, B: Schema {
    type Out = (A::Out, B::Out);

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if u.as_map().is_some() {
            return Some((self.0.find_deep(glob, u)?, self.1.find_deep(glob, u)?));
        }

        None
    }
}

impl<A, B> Schema for SchemaOr<A, B> where A: Schema, B: Schema {
    type Out = Or<A::Out, B::Out>;

    fn find(&self, glob: &Unit, u: &Unit) -> Option<Self::Out> {
        if let Some(v) = self.0.find_deep(glob, u) {
            return Some(Or::First(v));
        }
        Some(Or::Second(self.1.find_deep(glob, u)?))
    }
}
