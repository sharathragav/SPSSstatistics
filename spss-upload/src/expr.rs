//! Expressions used by Compute Variable and Select Cases.

use crate::data::Dataset;

#[derive(Clone, Debug)]
pub enum Val {
    Num(f64),
    Str(String),
}
impl Val {
    pub fn num(&self) -> f64 {
        match self {
            Val::Num(x) => *x,
            Val::Str(s) => s.trim().parse().unwrap_or(f64::NAN),
        }
    }
    pub fn truthy(&self) -> bool {
        let x = self.num();
        !x.is_nan() && x != 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(f64),
    Str(String),
    Var(usize),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Id(String),
    Op(Op),
    Not,
    Minus,
    LP,
    RP,
    Comma,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = vec![];
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < c.len() && c[i + 1].is_ascii_digit()) {
            let st = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            if i < c.len() && (c[i] == 'e' || c[i] == 'E') && i + 1 < c.len() && (c[i + 1].is_ascii_digit() || c[i + 1] == '-' || c[i + 1] == '+') {
                i += 2;
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let t: String = c[st..i].iter().collect();
            out.push(Tok::Num(t.parse().map_err(|_| format!("Bad number '{}'", t))?));
            continue;
        }
        if ch.is_alphabetic() || ch == '_' || ch == '@' || ch == '#' || ch == '$' {
            let st = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '@' || c[i] == '#' || c[i] == '$' || c[i] == '.') {
                i += 1;
            }
            let mut t: String = c[st..i].iter().collect();
            while t.ends_with('.') {
                t.pop();
                i -= 1;
            }
            let u = t.to_ascii_uppercase();
            out.push(match u.as_str() {
                "AND" => Tok::Op(Op::And),
                "OR" => Tok::Op(Op::Or),
                "NOT" => Tok::Not,
                "EQ" => Tok::Op(Op::Eq),
                "NE" => Tok::Op(Op::Ne),
                "LT" => Tok::Op(Op::Lt),
                "LE" => Tok::Op(Op::Le),
                "GT" => Tok::Op(Op::Gt),
                "GE" => Tok::Op(Op::Ge),
                _ => Tok::Id(t),
            });
            continue;
        }
        if ch == '\'' || ch == '"' {
            let q = ch;
            i += 1;
            let mut t = String::new();
            while i < c.len() && c[i] != q {
                t.push(c[i]);
                i += 1;
            }
            if i >= c.len() {
                return Err("Unclosed quote".into());
            }
            i += 1;
            out.push(Tok::Str(t));
            continue;
        }
        let two: String = c[i..(i + 2).min(c.len())].iter().collect();
        let t2 = match two.as_str() {
            "**" => Some(Tok::Op(Op::Pow)),
            "<=" => Some(Tok::Op(Op::Le)),
            ">=" => Some(Tok::Op(Op::Ge)),
            "<>" | "~=" | "!=" => Some(Tok::Op(Op::Ne)),
            "==" => Some(Tok::Op(Op::Eq)),
            _ => None,
        };
        if let Some(t) = t2 {
            out.push(t);
            i += 2;
            continue;
        }
        out.push(match ch {
            '+' => Tok::Op(Op::Add),
            '-' => Tok::Minus,
            '*' => Tok::Op(Op::Mul),
            '/' => Tok::Op(Op::Div),
            '^' => Tok::Op(Op::Pow),
            '=' => Tok::Op(Op::Eq),
            '<' => Tok::Op(Op::Lt),
            '>' => Tok::Op(Op::Gt),
            '&' => Tok::Op(Op::And),
            '|' => Tok::Op(Op::Or),
            '~' | '!' => Tok::Not,
            '(' => Tok::LP,
            ')' => Tok::RP,
            ',' => Tok::Comma,
            _ => return Err(format!("Unexpected character '{}'", ch)),
        });
        i += 1;
    }
    Ok(out)
}

struct P<'a> {
    t: Vec<Tok>,
    i: usize,
    ds: &'a Dataset,
}

impl<'a> P<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn or(&mut self) -> Result<Expr, String> {
        let mut a = self.and()?;
        while self.eat(&Tok::Op(Op::Or)) {
            let b = self.and()?;
            a = Expr::Bin(Op::Or, Box::new(a), Box::new(b));
        }
        Ok(a)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut a = self.not()?;
        while self.eat(&Tok::Op(Op::And)) {
            let b = self.not()?;
            a = Expr::Bin(Op::And, Box::new(a), Box::new(b));
        }
        Ok(a)
    }
    fn not(&mut self) -> Result<Expr, String> {
        if self.eat(&Tok::Not) {
            let a = self.not()?;
            return Ok(Expr::Not(Box::new(a)));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<Expr, String> {
        let a = self.add()?;
        if let Some(Tok::Op(op)) = self.peek().cloned() {
            if matches!(op, Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge) {
                self.i += 1;
                let b = self.add()?;
                return Ok(Expr::Bin(op, Box::new(a), Box::new(b)));
            }
        }
        Ok(a)
    }
    fn add(&mut self) -> Result<Expr, String> {
        let mut a = self.mul()?;
        loop {
            if self.eat(&Tok::Op(Op::Add)) {
                let b = self.mul()?;
                a = Expr::Bin(Op::Add, Box::new(a), Box::new(b));
            } else if self.eat(&Tok::Minus) {
                let b = self.mul()?;
                a = Expr::Bin(Op::Sub, Box::new(a), Box::new(b));
            } else {
                return Ok(a);
            }
        }
    }
    fn mul(&mut self) -> Result<Expr, String> {
        let mut a = self.unary()?;
        loop {
            if self.eat(&Tok::Op(Op::Mul)) {
                let b = self.unary()?;
                a = Expr::Bin(Op::Mul, Box::new(a), Box::new(b));
            } else if self.eat(&Tok::Op(Op::Div)) {
                let b = self.unary()?;
                a = Expr::Bin(Op::Div, Box::new(a), Box::new(b));
            } else {
                return Ok(a);
            }
        }
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat(&Tok::Minus) {
            let a = self.unary()?;
            return Ok(Expr::Neg(Box::new(a)));
        }
        self.pow()
    }
    fn pow(&mut self) -> Result<Expr, String> {
        let a = self.atom()?;
        if self.eat(&Tok::Op(Op::Pow)) {
            let b = self.unary()?;
            return Ok(Expr::Bin(Op::Pow, Box::new(a), Box::new(b)));
        }
        Ok(a)
    }
    fn atom(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::Num(x)) => {
                self.i += 1;
                Ok(Expr::Num(x))
            }
            Some(Tok::Str(s)) => {
                self.i += 1;
                Ok(Expr::Str(s))
            }
            Some(Tok::LP) => {
                self.i += 1;
                let e = self.or()?;
                if !self.eat(&Tok::RP) {
                    return Err("Missing ')'".into());
                }
                Ok(e)
            }
            Some(Tok::Id(name)) => {
                self.i += 1;
                if self.eat(&Tok::LP) {
                    let mut args = vec![];
                    if !self.eat(&Tok::RP) {
                        loop {
                            args.push(self.or()?);
                            if self.eat(&Tok::RP) {
                                break;
                            }
                            if !self.eat(&Tok::Comma) {
                                return Err(format!("Expected ',' or ')' in {}()", name));
                            }
                        }
                    }
                    let u = name.to_ascii_uppercase();
                    if !FUNCS.iter().any(|(f, _)| *f == u) {
                        return Err(format!("Unknown function '{}'", name));
                    }
                    Ok(Expr::Call(u, args))
                } else {
                    match self.ds.var_index(&name) {
                        Some(i) => Ok(Expr::Var(i)),
                        None => {
                            if name.eq_ignore_ascii_case("$CASENUM") {
                                Ok(Expr::Call("$CASENUM".into(), vec![]))
                            } else if name.eq_ignore_ascii_case("$SYSMIS") {
                                Ok(Expr::Num(f64::NAN))
                            } else {
                                Err(format!("Variable '{}' doesn't exist", name))
                            }
                        }
                    }
                }
            }
            Some(t) => Err(format!("Unexpected {:?}", t)),
            None => Err("The expression is incomplete".into()),
        }
    }
}

/// Functions offered in the Compute dialog: (name, description).
pub const FUNCS: &[(&str, &str)] = &[
    ("ABS", "ABS(x) absolute value"),
    ("SQRT", "SQRT(x) square root"),
    ("LN", "LN(x) natural log"),
    ("LG10", "LG10(x) base-10 log"),
    ("EXP", "EXP(x) e to the power x"),
    ("RND", "RND(x) round to whole number"),
    ("TRUNC", "TRUNC(x) drop the fraction"),
    ("MOD", "MOD(x, y) remainder"),
    ("MEAN", "MEAN(a, b, ...) mean of valid values"),
    ("SUM", "SUM(a, b, ...) sum of valid values"),
    ("MIN", "MIN(a, b, ...) smallest value"),
    ("MAX", "MAX(a, b, ...) largest value"),
    ("SD", "SD(a, b, ...) standard deviation"),
    ("VARIANCE", "VARIANCE(a, b, ...) variance"),
    ("NVALID", "NVALID(a, b, ...) count of valid values"),
    ("NMISS", "NMISS(a, b, ...) count of missing values"),
    ("MISSING", "MISSING(x) 1 if missing"),
    ("SYSMIS", "SYSMIS(x) 1 if system-missing"),
    ("RANGE", "RANGE(x, lo, hi) 1 if lo <= x <= hi"),
    ("UNIFORM", "UNIFORM(n) random number 0..n"),
    ("NORMAL", "NORMAL(sd) random normal value"),
    ("LENGTH", "LENGTH(s) characters in a string"),
    ("UPCASE", "UPCASE(s) upper-case text"),
    ("LOWER", "LOWER(s) lower-case text"),
    ("CONCAT", "CONCAT(a, b, ...) join text"),
    ("NUMBER", "NUMBER(s) text to number"),
    ("STRING", "STRING(x) number to text"),
    ("$CASENUM", "$CASENUM case number"),
];

pub fn parse(src: &str, ds: &Dataset) -> Result<Expr, String> {
    let t = tokenize(src)?;
    if t.is_empty() {
        return Err("The expression is empty".into());
    }
    let mut p = P { t, i: 0, ds };
    let e = p.or()?;
    if p.i < p.t.len() {
        return Err(format!("Unexpected {:?} after the expression", p.t[p.i]));
    }
    Ok(e)
}

pub struct Ctx {
    seed: u64,
}
impl Ctx {
    pub fn new() -> Self {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Ctx { seed: t | 1 }
    }
    fn rand(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn b(x: bool) -> Val {
    Val::Num(if x { 1.0 } else { 0.0 })
}

impl Expr {
    pub fn is_string(&self, ds: &Dataset) -> bool {
        match self {
            Expr::Str(_) => true,
            Expr::Var(i) => !ds.vars[*i].is_numeric(),
            Expr::Call(f, _) => matches!(f.as_str(), "UPCASE" | "LOWER" | "CONCAT" | "STRING"),
            _ => false,
        }
    }
    pub fn eval(&self, ds: &Dataset, r: usize, cx: &mut Ctx) -> Val {
        match self {
            Expr::Num(x) => Val::Num(*x),
            Expr::Str(s) => Val::Str(s.clone()),
            Expr::Var(i) => {
                let v = &ds.vars[*i];
                if v.is_numeric() {
                    Val::Num(v.valid_num(r).unwrap_or(f64::NAN))
                } else {
                    Val::Str(v.cell_text(r, false))
                }
            }
            Expr::Neg(a) => Val::Num(-a.eval(ds, r, cx).num()),
            Expr::Not(a) => {
                let x = a.eval(ds, r, cx).num();
                if x.is_nan() {
                    Val::Num(f64::NAN)
                } else {
                    b(x == 0.0)
                }
            }
            Expr::Bin(op, a, bb) => {
                let x = a.eval(ds, r, cx);
                let y = bb.eval(ds, r, cx);
                match op {
                    Op::And => {
                        let (p, q) = (x.num(), y.num());
                        if p == 0.0 || q == 0.0 {
                            b(false)
                        } else if p.is_nan() || q.is_nan() {
                            Val::Num(f64::NAN)
                        } else {
                            b(true)
                        }
                    }
                    Op::Or => {
                        let (p, q) = (x.num(), y.num());
                        if (!p.is_nan() && p != 0.0) || (!q.is_nan() && q != 0.0) {
                            b(true)
                        } else if p.is_nan() || q.is_nan() {
                            Val::Num(f64::NAN)
                        } else {
                            b(false)
                        }
                    }
                    Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                        let ord = match (&x, &y) {
                            (Val::Str(s), Val::Str(t)) => Some(s.trim().cmp(t.trim())),
                            _ => {
                                let (p, q) = (x.num(), y.num());
                                if p.is_nan() || q.is_nan() {
                                    None
                                } else {
                                    p.partial_cmp(&q)
                                }
                            }
                        };
                        match ord {
                            None => Val::Num(f64::NAN),
                            Some(o) => {
                                use std::cmp::Ordering::*;
                                b(match op {
                                    Op::Eq => o == Equal,
                                    Op::Ne => o != Equal,
                                    Op::Lt => o == Less,
                                    Op::Le => o != Greater,
                                    Op::Gt => o == Greater,
                                    _ => o != Less,
                                })
                            }
                        }
                    }
                    _ => {
                        let (p, q) = (x.num(), y.num());
                        Val::Num(match op {
                            Op::Add => p + q,
                            Op::Sub => p - q,
                            Op::Mul => p * q,
                            Op::Div => {
                                if q == 0.0 {
                                    f64::NAN
                                } else {
                                    p / q
                                }
                            }
                            _ => {
                                let z = p.powf(q);
                                if z.is_finite() {
                                    z
                                } else {
                                    f64::NAN
                                }
                            }
                        })
                    }
                }
            }
            Expr::Call(f, args) => {
                let vals: Vec<Val> = args.iter().map(|a| a.eval(ds, r, cx)).collect();
                let nums: Vec<f64> = vals.iter().map(|v| v.num()).collect();
                let valid: Vec<f64> = nums.iter().copied().filter(|x| !x.is_nan()).collect();
                let a0 = nums.first().copied().unwrap_or(f64::NAN);
                let nan = f64::NAN;
                let res = match f.as_str() {
                    "ABS" => a0.abs(),
                    "SQRT" => {
                        if a0 >= 0.0 {
                            a0.sqrt()
                        } else {
                            nan
                        }
                    }
                    "LN" => {
                        if a0 > 0.0 {
                            a0.ln()
                        } else {
                            nan
                        }
                    }
                    "LG10" => {
                        if a0 > 0.0 {
                            a0.log10()
                        } else {
                            nan
                        }
                    }
                    "EXP" => a0.exp(),
                    "RND" => {
                        if nums.len() > 1 && nums[1] > 0.0 {
                            (a0 / nums[1]).round() * nums[1]
                        } else {
                            a0.round()
                        }
                    }
                    "TRUNC" => a0.trunc(),
                    "MOD" => {
                        let q = nums.get(1).copied().unwrap_or(nan);
                        if q == 0.0 {
                            nan
                        } else {
                            a0 % q
                        }
                    }
                    "MEAN" => {
                        if valid.is_empty() {
                            nan
                        } else {
                            valid.iter().sum::<f64>() / valid.len() as f64
                        }
                    }
                    "SUM" => {
                        if valid.is_empty() {
                            nan
                        } else {
                            valid.iter().sum()
                        }
                    }
                    "MIN" => valid.iter().copied().fold(nan, f64::min),
                    "MAX" => valid.iter().copied().fold(nan, f64::max),
                    "SD" | "VARIANCE" => {
                        if valid.len() < 2 {
                            nan
                        } else {
                            let m = valid.iter().sum::<f64>() / valid.len() as f64;
                            let var = valid.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (valid.len() - 1) as f64;
                            if f == "SD" {
                                var.sqrt()
                            } else {
                                var
                            }
                        }
                    }
                    "NVALID" => valid.len() as f64,
                    "NMISS" => (nums.len() - valid.len()) as f64,
                    "MISSING" | "SYSMIS" => {
                        let miss = match args.first() {
                            Some(Expr::Var(i)) => {
                                if f == "SYSMIS" {
                                    ds.vars[*i].is_sysmis(r)
                                } else {
                                    ds.vars[*i].is_missing(r)
                                }
                            }
                            _ => a0.is_nan(),
                        };
                        if miss {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    "RANGE" => {
                        if nums.len() < 3 || a0.is_nan() {
                            nan
                        } else if a0 >= nums[1] && a0 <= nums[2] {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    "UNIFORM" => cx.rand() * a0,
                    "NORMAL" => {
                        let u1 = cx.rand().max(1e-12);
                        let u2 = cx.rand();
                        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * a0
                    }
                    "LENGTH" => match vals.first() {
                        Some(Val::Str(s)) => s.trim_end().chars().count() as f64,
                        _ => nan,
                    },
                    "NUMBER" => a0,
                    "$CASENUM" => (r + 1) as f64,
                    "UPCASE" | "LOWER" | "CONCAT" | "STRING" => {
                        let txt = |v: &Val| match v {
                            Val::Str(s) => s.clone(),
                            Val::Num(x) => {
                                if x.is_nan() {
                                    String::new()
                                } else {
                                    crate::data::fmt_key(*x)
                                }
                            }
                        };
                        let s = match f.as_str() {
                            "UPCASE" => vals.first().map(txt).unwrap_or_default().to_uppercase(),
                            "LOWER" => vals.first().map(txt).unwrap_or_default().to_lowercase(),
                            "STRING" => vals.first().map(txt).unwrap_or_default(),
                            _ => vals.iter().map(txt).collect::<Vec<_>>().join(""),
                        };
                        return Val::Str(s);
                    }
                    _ => nan,
                };
                Val::Num(if res.is_finite() { res } else { nan })
            }
        }
    }
}
