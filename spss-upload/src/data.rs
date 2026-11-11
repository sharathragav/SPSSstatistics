//! The dataset: variables (with their dictionary) and cases.

use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Measure {
    Scale,
    Ordinal,
    Nominal,
}
impl Measure {
    pub const ALL: [Measure; 3] = [Measure::Scale, Measure::Ordinal, Measure::Nominal];
    pub fn name(&self) -> &'static str {
        match self {
            Measure::Scale => "Scale",
            Measure::Ordinal => "Ordinal",
            Measure::Nominal => "Nominal",
        }
    }
    pub fn parse(s: &str) -> Measure {
        match s.to_ascii_lowercase().as_str() {
            "ordinal" => Measure::Ordinal,
            "nominal" => Measure::Nominal,
            _ => Measure::Scale,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Right,
    Center,
}
impl Align {
    pub const ALL: [Align; 3] = [Align::Left, Align::Right, Align::Center];
    pub fn name(&self) -> &'static str {
        match self {
            Align::Left => "Left",
            Align::Right => "Right",
            Align::Center => "Center",
        }
    }
    pub fn parse(s: &str) -> Align {
        match s.to_ascii_lowercase().as_str() {
            "left" => Align::Left,
            "center" => Align::Center,
            _ => Align::Right,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Input,
    Target,
    Both,
    None,
    Partition,
    Split,
}
impl Role {
    pub const ALL: [Role; 6] = [Role::Input, Role::Target, Role::Both, Role::None, Role::Partition, Role::Split];
    pub fn name(&self) -> &'static str {
        match self {
            Role::Input => "Input",
            Role::Target => "Target",
            Role::Both => "Both",
            Role::None => "None",
            Role::Partition => "Partition",
            Role::Split => "Split",
        }
    }
    pub fn parse(s: &str) -> Role {
        for r in Role::ALL {
            if r.name().eq_ignore_ascii_case(s) {
                return r;
            }
        }
        Role::Input
    }
}

#[derive(Clone, Debug)]
pub enum Col {
    Num(Vec<f64>),
    Str(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct Variable {
    pub name: String,
    pub width: usize,
    pub decimals: usize,
    pub label: String,
    /// value (as key text) -> label
    pub value_labels: Vec<(String, String)>,
    /// user-missing values (as key text)
    pub missing: Vec<String>,
    pub columns: usize,
    pub align: Align,
    pub measure: Measure,
    pub role: Role,
    pub col: Col,
}

/// Canonical text of a number, used as a key for grouping and value labels.
pub fn fmt_key(v: f64) -> String {
    if v.is_nan() {
        ".".into()
    } else if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

pub fn norm_key(s: &str, numeric: bool) -> String {
    let t = s.trim();
    if numeric {
        match t.parse::<f64>() {
            Ok(v) => fmt_key(v),
            Err(_) => t.to_string(),
        }
    } else {
        t.to_string()
    }
}

impl Variable {
    pub fn new_numeric(name: &str, n: usize) -> Self {
        Variable {
            name: name.to_string(),
            width: 8,
            decimals: 2,
            label: String::new(),
            value_labels: vec![],
            missing: vec![],
            columns: 8,
            align: Align::Right,
            measure: Measure::Scale,
            role: Role::Input,
            col: Col::Num(vec![f64::NAN; n]),
        }
    }
    pub fn new_string(name: &str, n: usize) -> Self {
        Variable {
            name: name.to_string(),
            width: 8,
            decimals: 0,
            label: String::new(),
            value_labels: vec![],
            missing: vec![],
            columns: 8,
            align: Align::Left,
            measure: Measure::Nominal,
            role: Role::Input,
            col: Col::Str(vec![String::new(); n]),
        }
    }
    pub fn is_numeric(&self) -> bool {
        matches!(self.col, Col::Num(_))
    }
    pub fn display_name(&self) -> String {
        if self.label.trim().is_empty() {
            self.name.clone()
        } else {
            format!("{} [{}]", self.label, self.name)
        }
    }
    /// Label if set, otherwise name (used for table titles).
    pub fn title(&self) -> String {
        if self.label.trim().is_empty() {
            self.name.clone()
        } else {
            self.label.clone()
        }
    }
    pub fn raw_num(&self, r: usize) -> f64 {
        match &self.col {
            Col::Num(v) => v.get(r).copied().unwrap_or(f64::NAN),
            Col::Str(s) => s.get(r).and_then(|x| x.trim().parse().ok()).unwrap_or(f64::NAN),
        }
    }
    pub fn key(&self, r: usize) -> String {
        match &self.col {
            Col::Num(v) => fmt_key(v.get(r).copied().unwrap_or(f64::NAN)),
            Col::Str(s) => s.get(r).map(|x| x.trim().to_string()).unwrap_or_default(),
        }
    }
    pub fn is_missing(&self, r: usize) -> bool {
        match &self.col {
            Col::Num(v) => {
                let x = v.get(r).copied().unwrap_or(f64::NAN);
                x.is_nan() || (!self.missing.is_empty() && self.missing.contains(&fmt_key(x)))
            }
            Col::Str(s) => {
                let x = s.get(r).map(|x| x.trim()).unwrap_or("");
                !self.missing.is_empty() && self.missing.iter().any(|m| m == x)
            }
        }
    }
    pub fn is_sysmis(&self, r: usize) -> bool {
        match &self.col {
            Col::Num(v) => v.get(r).map(|x| x.is_nan()).unwrap_or(true),
            Col::Str(_) => false,
        }
    }
    /// Valid numeric value (not system or user missing).
    pub fn valid_num(&self, r: usize) -> Option<f64> {
        if self.is_missing(r) {
            return None;
        }
        let v = self.raw_num(r);
        if v.is_nan() {
            None
        } else {
            Some(v)
        }
    }
    pub fn label_for_key(&self, key: &str) -> Option<&str> {
        self.value_labels.iter().find(|(k, _)| k == key).map(|(_, l)| l.as_str())
    }
    pub fn key_display(&self, key: &str) -> String {
        match self.label_for_key(key) {
            Some(l) => l.to_string(),
            None => {
                if self.is_numeric() {
                    match key.parse::<f64>() {
                        Ok(v) => self.format_value(v),
                        Err(_) => key.to_string(),
                    }
                } else {
                    key.to_string()
                }
            }
        }
    }
    pub fn format_value(&self, v: f64) -> String {
        if v.is_nan() {
            ".".into()
        } else {
            format!("{:.*}", self.decimals, v)
        }
    }
    pub fn cell_text(&self, r: usize, labels: bool) -> String {
        match &self.col {
            Col::Num(v) => {
                let x = v.get(r).copied().unwrap_or(f64::NAN);
                if labels && !x.is_nan() {
                    if let Some(l) = self.label_for_key(&fmt_key(x)) {
                        return l.to_string();
                    }
                }
                self.format_value(x)
            }
            Col::Str(s) => {
                let x = s.get(r).cloned().unwrap_or_default();
                if labels {
                    if let Some(l) = self.label_for_key(x.trim()) {
                        return l.to_string();
                    }
                }
                x
            }
        }
    }
    /// Text used when editing a cell (full precision).
    pub fn edit_text(&self, r: usize) -> String {
        match &self.col {
            Col::Num(v) => {
                let x = v.get(r).copied().unwrap_or(f64::NAN);
                if x.is_nan() {
                    String::new()
                } else {
                    fmt_key(x)
                }
            }
            Col::Str(s) => s.get(r).cloned().unwrap_or_default(),
        }
    }
    /// Sets a cell from typed text. Returns false if the text doesn't fit the type.
    pub fn set_text(&mut self, r: usize, text: &str) -> bool {
        let t = text.trim();
        match &mut self.col {
            Col::Num(v) => {
                if r >= v.len() {
                    return false;
                }
                if t.is_empty() || t == "." {
                    v[r] = f64::NAN;
                    return true;
                }
                // allow typing a value label
                if let Some((k, _)) = self.value_labels.iter().find(|(_, l)| l.eq_ignore_ascii_case(t)) {
                    if let Ok(x) = k.parse::<f64>() {
                        v[r] = x;
                        return true;
                    }
                }
                match t.replace(',', "").parse::<f64>() {
                    Ok(x) => {
                        v[r] = x;
                        true
                    }
                    Err(_) => false,
                }
            }
            Col::Str(s) => {
                if r >= s.len() {
                    return false;
                }
                s[r] = text.to_string();
                if text.len() > self.width {
                    self.width = text.len();
                }
                true
            }
        }
    }
    pub fn set_num(&mut self, r: usize, x: f64) {
        match &mut self.col {
            Col::Num(v) => {
                if r < v.len() {
                    v[r] = x;
                }
            }
            Col::Str(s) => {
                if r < s.len() {
                    s[r] = if x.is_nan() { String::new() } else { fmt_key(x) };
                }
            }
        }
    }
    pub fn len(&self) -> usize {
        match &self.col {
            Col::Num(v) => v.len(),
            Col::Str(s) => s.len(),
        }
    }
    pub fn push_case(&mut self) {
        match &mut self.col {
            Col::Num(v) => v.push(f64::NAN),
            Col::Str(s) => s.push(String::new()),
        }
    }
    pub fn insert_case(&mut self, at: usize) {
        match &mut self.col {
            Col::Num(v) => v.insert(at.min(v.len()), f64::NAN),
            Col::Str(s) => s.insert(at.min(s.len()), String::new()),
        }
    }
    pub fn remove_case(&mut self, at: usize) {
        match &mut self.col {
            Col::Num(v) => {
                if at < v.len() {
                    v.remove(at);
                }
            }
            Col::Str(s) => {
                if at < s.len() {
                    s.remove(at);
                }
            }
        }
    }
    pub fn set_numeric(&mut self, numeric: bool) {
        if numeric == self.is_numeric() {
            return;
        }
        match &self.col {
            Col::Num(v) => {
                let s: Vec<String> = v.iter().map(|x| if x.is_nan() { String::new() } else { fmt_key(*x) }).collect();
                self.col = Col::Str(s);
                self.align = Align::Left;
                self.decimals = 0;
                self.measure = Measure::Nominal;
            }
            Col::Str(s) => {
                let v: Vec<f64> = s.iter().map(|x| x.trim().parse().unwrap_or(f64::NAN)).collect();
                self.col = Col::Num(v);
                self.align = Align::Right;
                self.decimals = 2;
            }
        }
    }
    pub fn permute(&mut self, order: &[usize]) {
        match &mut self.col {
            Col::Num(v) => {
                let nv: Vec<f64> = order.iter().map(|&i| v[i]).collect();
                *v = nv;
            }
            Col::Str(s) => {
                let ns: Vec<String> = order.iter().map(|&i| s[i].clone()).collect();
                *s = ns;
            }
        }
    }
    pub fn type_name(&self) -> &'static str {
        if self.is_numeric() {
            "Numeric"
        } else {
            "String"
        }
    }
    pub fn values_summary(&self) -> String {
        if self.value_labels.is_empty() {
            "None".into()
        } else {
            let (k, l) = &self.value_labels[0];
            format!("{{{}, {}}}...", k, l)
        }
    }
    pub fn missing_summary(&self) -> String {
        if self.missing.is_empty() {
            "None".into()
        } else {
            self.missing.join(", ")
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Dataset {
    pub vars: Vec<Variable>,
    pub ncases: usize,
    pub filter: Option<Vec<bool>>,
    pub filter_expr: String,
}

pub fn valid_name(s: &str) -> String {
    let mut out: String = s
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '@' || c == '#' || c == '$' { c } else { '_' })
        .collect();
    while out.ends_with('_') && out.len() > 1 {
        out.pop();
    }
    if out.is_empty() {
        out = "v".into();
    }
    if !out.chars().next().map(|c| c.is_alphabetic() || c == '@').unwrap_or(false) {
        out = format!("v{}", out);
    }
    if out.len() > 64 {
        out.truncate(64);
    }
    out
}

impl Dataset {
    pub fn var_index(&self, name: &str) -> Option<usize> {
        self.vars.iter().position(|v| v.name.eq_ignore_ascii_case(name.trim()))
    }
    pub fn unique_name(&self, base: &str, except: Option<usize>) -> String {
        let base = valid_name(base);
        let taken = |n: &str| self.vars.iter().enumerate().any(|(i, v)| Some(i) != except && v.name.eq_ignore_ascii_case(n));
        if !taken(&base) {
            return base;
        }
        for i in 1.. {
            let n = format!("{}_{}", base, i);
            if !taken(&n) {
                return n;
            }
        }
        base
    }
    pub fn next_var_name(&self) -> String {
        for i in 1.. {
            let n = format!("VAR{:05}", i);
            if self.var_index(&n).is_none() {
                return n;
            }
        }
        "VAR".into()
    }
    pub fn add_var(&mut self, mut v: Variable) -> usize {
        while v.len() < self.ncases {
            v.push_case();
        }
        self.vars.push(v);
        self.vars.len() - 1
    }
    pub fn insert_var(&mut self, at: usize) {
        let name = self.next_var_name();
        let v = Variable::new_numeric(&name, self.ncases);
        let at = at.min(self.vars.len());
        self.vars.insert(at, v);
    }
    pub fn ensure_vars(&mut self, upto: usize) {
        while self.vars.len() <= upto {
            let name = self.next_var_name();
            let v = Variable::new_numeric(&name, self.ncases);
            self.vars.push(v);
        }
    }
    pub fn ensure_cases(&mut self, upto: usize) {
        while self.ncases <= upto {
            for v in &mut self.vars {
                v.push_case();
            }
            if let Some(f) = &mut self.filter {
                f.push(true);
            }
            self.ncases += 1;
        }
    }
    pub fn insert_case(&mut self, at: usize) {
        let at = at.min(self.ncases);
        for v in &mut self.vars {
            v.insert_case(at);
        }
        if let Some(f) = &mut self.filter {
            f.insert(at.min(f.len()), true);
        }
        self.ncases += 1;
    }
    pub fn delete_case(&mut self, at: usize) {
        if at >= self.ncases {
            return;
        }
        for v in &mut self.vars {
            v.remove_case(at);
        }
        if let Some(f) = &mut self.filter {
            if at < f.len() {
                f.remove(at);
            }
        }
        self.ncases -= 1;
    }
    pub fn delete_var(&mut self, at: usize) {
        if at < self.vars.len() {
            self.vars.remove(at);
        }
    }
    pub fn set_cell(&mut self, r: usize, c: usize, text: &str) -> bool {
        if text.trim().is_empty() && (c >= self.vars.len() || r >= self.ncases) {
            return true;
        }
        let creating = c >= self.vars.len();
        self.ensure_vars(c);
        self.ensure_cases(r);
        if creating && text.trim().parse::<f64>().is_err() && text.trim() != "." {
            // a new column receiving text becomes a string variable
            let v = &mut self.vars[c];
            v.set_numeric(false);
        }
        self.vars[c].set_text(r, text)
    }
    pub fn included(&self, r: usize) -> bool {
        match &self.filter {
            Some(f) => f.get(r).copied().unwrap_or(true),
            None => true,
        }
    }
    pub fn rows(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.ncases).filter(move |&r| self.included(r))
    }
    pub fn sort_by(&mut self, keys: &[(usize, bool)]) {
        let mut order: Vec<usize> = (0..self.ncases).collect();
        order.sort_by(|&a, &b| {
            for &(vi, asc) in keys {
                let v = &self.vars[vi];
                let o = match &v.col {
                    Col::Num(x) => {
                        let (p, q) = (x[a], x[b]);
                        match (p.is_nan(), q.is_nan()) {
                            (true, true) => std::cmp::Ordering::Equal,
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => p.partial_cmp(&q).unwrap_or(std::cmp::Ordering::Equal),
                        }
                    }
                    Col::Str(s) => s[a].cmp(&s[b]),
                };
                let o = if asc { o } else { o.reverse() };
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            std::cmp::Ordering::Equal
        });
        for v in &mut self.vars {
            v.permute(&order);
        }
        if let Some(f) = &mut self.filter {
            let nf: Vec<bool> = order.iter().map(|&i| f.get(i).copied().unwrap_or(true)).collect();
            *f = nf;
        }
    }
    /// Distinct valid keys of a variable over included cases, sorted.
    pub fn categories(&self, vi: usize) -> Vec<String> {
        let v = &self.vars[vi];
        let mut keys: Vec<String> = vec![];
        for r in self.rows() {
            if v.is_missing(r) {
                continue;
            }
            let k = v.key(r);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        sort_keys(&mut keys, v.is_numeric());
        keys
    }
}

pub fn sort_keys(keys: &mut [String], numeric: bool) {
    if numeric {
        keys.sort_by(|a, b| {
            let x: f64 = a.parse().unwrap_or(f64::NAN);
            let y: f64 = b.parse().unwrap_or(f64::NAN);
            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        keys.sort();
    }
}

// ---------------------------------------------------------------- files

pub fn parse_delimited(text: &str, delim: char) -> Vec<Vec<String>> {
    let mut rows = vec![];
    let mut row: Vec<String> = vec![];
    let mut field = String::new();
    let mut in_q = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_q {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_q = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && field.trim().is_empty() {
            field.clear();
            in_q = true;
        } else if c == delim {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut field));
            if !(row.len() == 1 && row[0].is_empty()) {
                rows.push(std::mem::take(&mut row));
            } else {
                row.clear();
            }
        } else {
            field.push(c);
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

fn detect_delim(text: &str) -> char {
    let first = text.lines().next().unwrap_or("");
    let cands = [',', '\t', ';', '|'];
    let mut best = ',';
    let mut bc = 0;
    for c in cands {
        let n = first.matches(c).count();
        if n > bc {
            bc = n;
            best = c;
        }
    }
    best
}

pub fn dataset_from_rows(rows: Vec<Vec<String>>, header: bool) -> Dataset {
    let mut ds = Dataset::default();
    if rows.is_empty() {
        return ds;
    }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let (names, body): (Vec<String>, &[Vec<String>]) = if header {
        (rows[0].clone(), &rows[1..])
    } else {
        ((0..ncols).map(|i| format!("VAR{:05}", i + 1)).collect(), &rows[..])
    };
    let n = body.len();
    for c in 0..ncols {
        let raw_name = names.get(c).cloned().unwrap_or_default();
        let name = if raw_name.trim().is_empty() { ds.next_var_name() } else { ds.unique_name(&raw_name, None) };
        let cells: Vec<&str> = body.iter().map(|r| r.get(c).map(|s| s.trim()).unwrap_or("")).collect();
        let numeric = cells.iter().all(|s| s.is_empty() || *s == "." || s.parse::<f64>().is_ok());
        let mut v = if numeric {
            let mut v = Variable::new_numeric(&name, 0);
            let mut dec = 0usize;
            let vals: Vec<f64> = cells
                .iter()
                .map(|s| {
                    if let Some(p) = s.find('.') {
                        dec = dec.max(s.len() - p - 1);
                    }
                    s.parse::<f64>().unwrap_or(f64::NAN)
                })
                .collect();
            v.decimals = dec.min(6);
            let mut distinct: Vec<i64> = vec![];
            let mut all_int = true;
            for x in vals.iter().filter(|x| !x.is_nan()) {
                if x.fract() != 0.0 {
                    all_int = false;
                    break;
                }
                let k = *x as i64;
                if !distinct.contains(&k) {
                    distinct.push(k);
                    if distinct.len() > 12 {
                        break;
                    }
                }
            }
            v.measure = if all_int && distinct.len() <= 7 && !distinct.is_empty() { Measure::Nominal } else { Measure::Scale };
            v.col = Col::Num(vals);
            v
        } else {
            let mut v = Variable::new_string(&name, 0);
            let w = cells.iter().map(|s| s.chars().count()).max().unwrap_or(8).max(1);
            v.width = w;
            v.columns = w.clamp(8, 30);
            v.col = Col::Str(cells.iter().map(|s| s.to_string()).collect());
            v
        };
        if raw_name.trim() != name && !raw_name.trim().is_empty() {
            v.label = raw_name.trim().to_string();
        }
        v.columns = v.columns.max(name.len().min(20));
        ds.vars.push(v);
    }
    ds.ncases = n;
    ds
}

fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\t' => o.push_str("\\t"),
            '\n' => o.push_str("\\n"),
            '\r' => {}
            '|' => o.push_str("\\p"),
            '=' => o.push_str("\\e"),
            _ => o.push(c),
        }
    }
    o
}
fn unesc(s: &str) -> String {
    let mut o = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('t') => o.push('\t'),
                Some('n') => o.push('\n'),
                Some('p') => o.push('|'),
                Some('e') => o.push('='),
                Some(x) => o.push(x),
                None => {}
            }
        } else {
            o.push(c);
        }
    }
    o
}

pub const NATIVE_EXT: &str = "sdv";

pub fn save_native(ds: &Dataset, path: &Path) -> Result<(), String> {
    let mut out = String::from("STATDESK-DATA\t1\n");
    for v in &ds.vars {
        let labels: Vec<String> = v.value_labels.iter().map(|(k, l)| format!("{}={}", esc(k), esc(l))).collect();
        let miss: Vec<String> = v.missing.iter().map(|m| esc(m)).collect();
        out.push_str(&format!(
            "VAR\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            esc(&v.name),
            if v.is_numeric() { "N" } else { "S" },
            v.width,
            v.decimals,
            esc(&v.label),
            v.columns,
            v.align.name(),
            v.measure.name(),
            v.role.name(),
            miss.join("|"),
            labels.join("|")
        ));
    }
    out.push_str("DATA\n");
    for r in 0..ds.ncases {
        let cells: Vec<String> = ds
            .vars
            .iter()
            .map(|v| match &v.col {
                Col::Num(x) => {
                    if x[r].is_nan() {
                        String::new()
                    } else {
                        format!("{}", x[r])
                    }
                }
                Col::Str(s) => esc(&s[r]),
            })
            .collect();
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
    std::fs::write(path, out).map_err(|e| e.to_string())
}

fn load_native(text: &str) -> Result<Dataset, String> {
    let mut ds = Dataset::default();
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("");
    if !head.starts_with("STATDESK-DATA") {
        return Err("Not a StatDesk data file.".into());
    }
    let mut in_data = false;
    let mut n = 0;
    for line in lines {
        if !in_data {
            if line == "DATA" {
                in_data = true;
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 12 || f[0] != "VAR" {
                continue;
            }
            let name = unesc(f[1]);
            let mut v = if f[2] == "S" { Variable::new_string(&name, 0) } else { Variable::new_numeric(&name, 0) };
            v.width = f[3].parse().unwrap_or(8);
            v.decimals = f[4].parse().unwrap_or(2);
            v.label = unesc(f[5]);
            v.columns = f[6].parse().unwrap_or(8);
            v.align = Align::parse(f[7]);
            v.measure = Measure::parse(f[8]);
            v.role = Role::parse(f[9]);
            v.missing = if f[10].is_empty() { vec![] } else { f[10].split('|').map(unesc).collect() };
            v.value_labels = if f[11].is_empty() {
                vec![]
            } else {
                f[11]
                    .split('|')
                    .filter_map(|p| {
                        let mut it = p.splitn(2, '=');
                        Some((unesc(it.next()?), unesc(it.next().unwrap_or(""))))
                    })
                    .collect()
            };
            ds.vars.push(v);
        } else {
            let f: Vec<&str> = line.split('\t').collect();
            for (i, v) in ds.vars.iter_mut().enumerate() {
                let cell = f.get(i).copied().unwrap_or("");
                match &mut v.col {
                    Col::Num(x) => x.push(cell.trim().parse().unwrap_or(f64::NAN)),
                    Col::Str(s) => s.push(unesc(cell)),
                }
            }
            n += 1;
        }
    }
    ds.ncases = n;
    Ok(ds)
}

pub fn load_file(path: &Path) -> Result<Dataset, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    let text = text.trim_start_matches('\u{feff}');
    if text.starts_with("STATDESK-DATA") {
        return load_native(text);
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let delim = if ext == "tsv" || ext == "tab" { '\t' } else { detect_delim(text) };
    let rows = parse_delimited(text, delim);
    if rows.is_empty() {
        return Err("The file is empty.".into());
    }
    // header if the first row has any non-numeric cell
    let header = rows[0].iter().any(|s| !s.trim().is_empty() && s.trim().parse::<f64>().is_err());
    Ok(dataset_from_rows(rows, header))
}

fn csv_field(s: &str, delim: char) -> String {
    if s.contains(delim) || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn save_csv(ds: &Dataset, path: &Path, labels: bool) -> Result<(), String> {
    let delim = if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("tsv")).unwrap_or(false) { '\t' } else { ',' };
    let mut out = String::new();
    let names: Vec<String> = ds.vars.iter().map(|v| csv_field(&v.name, delim)).collect();
    out.push_str(&names.join(&delim.to_string()));
    out.push('\n');
    for r in 0..ds.ncases {
        let cells: Vec<String> = ds
            .vars
            .iter()
            .map(|v| {
                let t = if labels {
                    v.cell_text(r, true)
                } else {
                    v.edit_text(r)
                };
                let t = if t == "." { String::new() } else { t };
                csv_field(&t, delim)
            })
            .collect();
        out.push_str(&cells.join(&delim.to_string()));
        out.push('\n');
    }
    std::fs::write(path, out).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- sample data

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let u1 = self.next().max(1e-12);
        let u2 = self.next();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    fn int(&mut self, lo: i64, hi: i64) -> f64 {
        (lo + (self.next() * (hi - lo + 1) as f64).floor() as i64).min(hi) as f64
    }
}

fn labels(pairs: &[(i32, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect()
}

/// A made-up workplace survey with 150 respondents, for trying things out.
pub fn sample_dataset() -> Dataset {
    let n = 150;
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut ds = Dataset::default();
    let mut id = Variable::new_numeric("id", 0);
    id.decimals = 0;
    id.label = "Respondent ID".into();
    let mut gender = Variable::new_numeric("gender", 0);
    gender.decimals = 0;
    gender.label = "Gender".into();
    gender.measure = Measure::Nominal;
    gender.value_labels = labels(&[(1, "Male"), (2, "Female")]);
    let mut age = Variable::new_numeric("age", 0);
    age.decimals = 0;
    age.label = "Age in years".into();
    let mut dept = Variable::new_numeric("dept", 0);
    dept.decimals = 0;
    dept.label = "Department".into();
    dept.measure = Measure::Nominal;
    dept.value_labels = labels(&[(1, "Sales"), (2, "Engineering"), (3, "Support"), (4, "Admin")]);
    let mut educ = Variable::new_numeric("educ", 0);
    educ.decimals = 0;
    educ.label = "Years of education".into();
    let mut salary = Variable::new_numeric("salary", 0);
    salary.decimals = 0;
    salary.label = "Annual salary".into();
    salary.columns = 9;
    let mut satisf = Variable::new_numeric("satisf", 0);
    satisf.decimals = 0;
    satisf.label = "Job satisfaction".into();
    satisf.measure = Measure::Ordinal;
    satisf.value_labels = labels(&[(1, "Very dissatisfied"), (2, "Dissatisfied"), (3, "Neutral"), (4, "Satisfied"), (5, "Very satisfied")]);
    satisf.missing = vec!["9".into()];
    let mut hours = Variable::new_numeric("hours", 0);
    hours.decimals = 1;
    hours.label = "Weekly hours worked".into();
    let mut pre = Variable::new_numeric("pretest", 0);
    pre.decimals = 1;
    pre.label = "Skills test before training".into();
    let mut post = Variable::new_numeric("posttest", 0);
    post.decimals = 1;
    post.label = "Skills test after training".into();
    let mut items: Vec<Variable> = (1..=4)
        .map(|i| {
            let mut v = Variable::new_numeric(&format!("q{}", i), 0);
            v.decimals = 0;
            v.measure = Measure::Ordinal;
            v.label = ["I enjoy my work", "My team supports me", "I feel valued", "I would recommend this employer"][i - 1].into();
            v.value_labels = labels(&[(1, "Strongly disagree"), (2, "Disagree"), (3, "Neutral"), (4, "Agree"), (5, "Strongly agree")]);
            v.col = Col::Num(vec![]);
            v
        })
        .collect();
    let mut cols: Vec<Vec<f64>> = vec![vec![]; 10];
    let mut qcols: Vec<Vec<f64>> = vec![vec![]; 4];
    for i in 0..n {
        let g = rng.int(1, 2);
        let a = (22.0 + rng.next() * 42.0).floor();
        let d = rng.int(1, 4);
        let e = (12.0 + rng.normal() * 2.2 + if d == 2.0 { 2.5 } else { 0.0 }).round().clamp(9.0, 21.0);
        let s = (18000.0 + e * 2100.0 + a * 380.0 + if d == 2.0 { 9000.0 } else { 0.0 } + rng.normal() * 6500.0).round();
        let latent = rng.normal();
        let sat = (3.2 + latent * 1.0 + (s - 50000.0) / 40000.0).round().clamp(1.0, 5.0);
        let h = ((40.0 + rng.normal() * 5.0 + if d == 2.0 { 2.0 } else { 0.0 }) * 10.0).round() / 10.0;
        let p = ((60.0 + rng.normal() * 9.0) * 10.0).round() / 10.0;
        let q = ((p + 4.5 + rng.normal() * 5.0) * 10.0).round() / 10.0;
        cols[0].push((i + 1) as f64);
        cols[1].push(g);
        cols[2].push(a);
        cols[3].push(d);
        cols[4].push(e);
        cols[5].push(if i == 17 || i == 88 { f64::NAN } else { s });
        cols[6].push(if i == 40 || i == 101 { 9.0 } else if i == 130 { f64::NAN } else { sat });
        cols[7].push(h);
        cols[8].push(p);
        cols[9].push(q);
        for (j, qc) in qcols.iter_mut().enumerate() {
            let x = (3.0 + latent * 0.9 + rng.normal() * 0.7 + if j == 3 { 0.3 } else { 0.0 }).round().clamp(1.0, 5.0);
            qc.push(x);
        }
    }
    let mut vars = vec![id, gender, age, dept, educ, salary, satisf, hours, pre, post];
    for (v, c) in vars.iter_mut().zip(cols) {
        v.col = Col::Num(c);
    }
    for (v, c) in items.iter_mut().zip(qcols) {
        v.col = Col::Num(c);
    }
    vars.extend(items);
    ds.vars = vars;
    ds.ncases = n;
    ds
}
