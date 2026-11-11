//! Commands: what every dialog produces, their text form, and the parser for the syntax editor.

use crate::data::Dataset;

#[derive(Clone, Debug)]
pub enum Command {
    Descriptives { vars: Vec<String>, stats: Vec<String>, save: bool },
    Frequencies { vars: Vec<String>, table: bool, stats: Vec<String>, chart: String, normal: bool },
    Crosstabs { rows: Vec<String>, cols: Vec<String>, cells: Vec<String>, chisq: bool, phi: bool },
    Means { deps: Vec<String>, by: String },
    TTestOne { vars: Vec<String>, value: f64 },
    TTestInd { vars: Vec<String>, group: String, g1: String, g2: String },
    TTestPaired { pairs: Vec<(String, String)> },
    Oneway { deps: Vec<String>, factor: String, desc: bool, homog: bool, bonf: bool },
    Correlations { vars: Vec<String>, spearman: bool, flag: bool },
    Regression { dep: String, ind: Vec<String>, desc: bool },
    Reliability { vars: Vec<String>, item_total: bool },
    MannWhitney { vars: Vec<String>, group: String, g1: String, g2: String },
    ChiSquare { vars: Vec<String> },
    Histogram { var: String, normal: bool },
    Bar { cat: String, mean_of: Option<String> },
    Scatter { x: String, y: String, fit: bool },
    Boxplot { var: String, by: Option<String> },
    Pie { cat: String },
    Compute { target: String, expr: String },
    Sort { keys: Vec<(String, bool)> },
    Filter { expr: Option<String> },
    Display { vars: Vec<String> },
}

fn join(v: &[String]) -> String {
    v.join(" ")
}

impl Command {
    pub fn title(&self) -> &'static str {
        match self {
            Command::Descriptives { .. } => "Descriptives",
            Command::Frequencies { .. } => "Frequencies",
            Command::Crosstabs { .. } => "Crosstabs",
            Command::Means { .. } => "Means",
            Command::TTestOne { .. } | Command::TTestInd { .. } | Command::TTestPaired { .. } => "T-Test",
            Command::Oneway { .. } => "Oneway",
            Command::Correlations { spearman, .. } => {
                if *spearman {
                    "Nonparametric Correlations"
                } else {
                    "Correlations"
                }
            }
            Command::Regression { .. } => "Regression",
            Command::Reliability { .. } => "Reliability",
            Command::MannWhitney { .. } | Command::ChiSquare { .. } => "NPar Tests",
            Command::Histogram { .. } | Command::Bar { .. } | Command::Scatter { .. } | Command::Pie { .. } => "Graph",
            Command::Boxplot { .. } => "Explore",
            Command::Compute { .. } => "Compute Variable",
            Command::Sort { .. } => "Sort Cases",
            Command::Filter { .. } => "Select Cases",
            Command::Display { .. } => "Variables",
        }
    }
    pub fn changes_data(&self) -> bool {
        matches!(self, Command::Compute { .. } | Command::Sort { .. } | Command::Filter { .. } | Command::Descriptives { save: true, .. })
    }
    pub fn to_syntax(&self) -> String {
        match self {
            Command::Descriptives { vars, stats, save } => format!("DESCRIPTIVES VARIABLES={}{}\n  /STATISTICS={}.", join(vars), if *save { "\n  /SAVE" } else { "" }, join(stats)),
            Command::Frequencies { vars, table, stats, chart, normal } => {
                let mut s = format!("FREQUENCIES VARIABLES={}", join(vars));
                if !table {
                    s.push_str("\n  /FORMAT=NOTABLE");
                }
                if !stats.is_empty() {
                    s.push_str(&format!("\n  /STATISTICS={}", join(stats)));
                }
                match chart.as_str() {
                    "BAR" => s.push_str("\n  /BARCHART FREQ"),
                    "PIE" => s.push_str("\n  /PIECHART FREQ"),
                    "HISTOGRAM" => s.push_str(if *normal { "\n  /HISTOGRAM NORMAL" } else { "\n  /HISTOGRAM" }),
                    _ => {}
                }
                s.push_str("\n  /ORDER=ANALYSIS.");
                s
            }
            Command::Crosstabs { rows, cols, cells, chisq, phi } => {
                let mut st = vec![];
                if *chisq {
                    st.push("CHISQ");
                }
                if *phi {
                    st.push("PHI");
                }
                let mut s = format!("CROSSTABS\n  /TABLES={} BY {}\n  /FORMAT=AVALUE TABLES", join(rows), join(cols));
                if !st.is_empty() {
                    s.push_str(&format!("\n  /STATISTICS={}", st.join(" ")));
                }
                s.push_str(&format!("\n  /CELLS={}\n  /COUNT ROUND CELL.", join(cells)));
                s
            }
            Command::Means { deps, by } => format!("MEANS TABLES={} BY {}\n  /CELLS=MEAN COUNT STDDEV.", join(deps), by),
            Command::TTestOne { vars, value } => format!("T-TEST\n  /TESTVAL={}\n  /MISSING=ANALYSIS\n  /VARIABLES={}\n  /CRITERIA=CI(.95).", crate::data::fmt_key(*value), join(vars)),
            Command::TTestInd { vars, group, g1, g2 } => format!("T-TEST GROUPS={}({} {})\n  /MISSING=ANALYSIS\n  /VARIABLES={}\n  /CRITERIA=CI(.95).", group, quote_if(g1), quote_if(g2), join(vars)),
            Command::TTestPaired { pairs } => {
                let a: Vec<String> = pairs.iter().map(|p| p.0.clone()).collect();
                let b: Vec<String> = pairs.iter().map(|p| p.1.clone()).collect();
                format!("T-TEST PAIRS={} WITH {} (PAIRED)\n  /CRITERIA=CI(.9500)\n  /MISSING=ANALYSIS.", join(&a), join(&b))
            }
            Command::Oneway { deps, factor, desc, homog, bonf } => {
                let mut s = format!("ONEWAY {} BY {}", join(deps), factor);
                let mut st = vec![];
                if *desc {
                    st.push("DESCRIPTIVES");
                }
                if *homog {
                    st.push("HOMOGENEITY");
                }
                if !st.is_empty() {
                    s.push_str(&format!("\n  /STATISTICS {}", st.join(" ")));
                }
                s.push_str("\n  /MISSING ANALYSIS");
                if *bonf {
                    s.push_str("\n  /POSTHOC=BONFERRONI ALPHA(0.05)");
                }
                s.push('.');
                s
            }
            Command::Correlations { vars, spearman, flag } => {
                if *spearman {
                    format!("NONPAR CORR\n  /VARIABLES={}\n  /PRINT=SPEARMAN TWOTAIL {}\n  /MISSING=PAIRWISE.", join(vars), if *flag { "SIG" } else { "NOSIG" })
                } else {
                    format!("CORRELATIONS\n  /VARIABLES={}\n  /PRINT=TWOTAIL {}\n  /MISSING=PAIRWISE.", join(vars), if *flag { "SIG" } else { "NOSIG" })
                }
            }
            Command::Regression { dep, ind, desc } => format!("REGRESSION{}\n  /DEPENDENT {}\n  /METHOD=ENTER {}.", if *desc { "\n  /DESCRIPTIVES MEAN STDDEV" } else { "" }, dep, join(ind)),
            Command::Reliability { vars, item_total } => format!("RELIABILITY\n  /VARIABLES={}\n  /MODEL=ALPHA{}.", join(vars), if *item_total { "\n  /SUMMARY=TOTAL" } else { "" }),
            Command::MannWhitney { vars, group, g1, g2 } => format!("NPAR TESTS\n  /M-W= {} BY {}({} {})\n  /MISSING ANALYSIS.", join(vars), group, quote_if(g1), quote_if(g2)),
            Command::ChiSquare { vars } => format!("NPAR TESTS\n  /CHISQUARE={}\n  /EXPECTED=EQUAL\n  /MISSING ANALYSIS.", join(vars)),
            Command::Histogram { var, normal } => format!("GRAPH\n  /HISTOGRAM{}={}.", if *normal { "(NORMAL)" } else { "" }, var),
            Command::Bar { cat, mean_of } => match mean_of {
                None => format!("GRAPH\n  /BAR(SIMPLE)=COUNT BY {}.", cat),
                Some(m) => format!("GRAPH\n  /BAR(SIMPLE)=MEAN({}) BY {}.", m, cat),
            },
            Command::Scatter { x, y, fit } => format!("GRAPH\n  /SCATTERPLOT(BIVAR)={} WITH {}{}.", x, y, if *fit { "\n  /FITLINE" } else { "" }),
            Command::Boxplot { var, by } => match by {
                None => format!("EXAMINE VARIABLES={}\n  /PLOT=BOXPLOT\n  /STATISTICS=NONE.", var),
                Some(b) => format!("EXAMINE VARIABLES={} BY {}\n  /PLOT=BOXPLOT\n  /STATISTICS=NONE.", var, b),
            },
            Command::Pie { cat } => format!("GRAPH\n  /PIE=COUNT BY {}.", cat),
            Command::Compute { target, expr } => format!("COMPUTE {}={}.\nEXECUTE.", target, expr),
            Command::Sort { keys } => {
                let k: Vec<String> = keys.iter().map(|(n, a)| format!("{}({})", n, if *a { "A" } else { "D" })).collect();
                format!("SORT CASES BY {}.", k.join(" "))
            }
            Command::Filter { expr } => match expr {
                Some(e) => format!("FILTER IF {}.", e),
                None => "USE ALL.\nFILTER OFF.".into(),
            },
            Command::Display { vars } => format!("DISPLAY DICTIONARY\n  /VARIABLES={}.", join(vars)),
        }
    }
}

fn quote_if(s: &str) -> String {
    if s.trim().parse::<f64>().is_ok() {
        s.trim().to_string()
    } else {
        format!("'{}'", s.trim().replace('\'', "''"))
    }
}

// ---------------------------------------------------------------- parser

/// Splits a script into commands (each ending with a period at end of line).
pub fn split_commands(text: &str) -> Vec<(usize, String)> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut start = 0usize;
    let mut q: Option<char> = None;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut pos = 0usize; // byte offset
    while i < chars.len() {
        let c = chars[i];
        if cur.trim().is_empty() && !c.is_whitespace() {
            start = pos;
        }
        cur.push(c);
        pos += c.len_utf8();
        match q {
            Some(qc) => {
                if c == qc {
                    q = None;
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    q = Some(c);
                } else if c == '.' {
                    // terminator if only whitespace follows on this line
                    let mut j = i + 1;
                    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t' || chars[j] == '\r') {
                        j += 1;
                    }
                    if j >= chars.len() || chars[j] == '\n' {
                        cur.pop();
                        if !cur.trim().is_empty() {
                            out.push((start, cur.trim().to_string()));
                        }
                        cur.clear();
                    }
                }
            }
        }
        i += 1;
    }
    if !cur.trim().is_empty() {
        out.push((start, cur.trim().to_string()));
    }
    out
}

fn var_list(s: &str, ds: &Dataset) -> Result<Vec<String>, String> {
    let toks: Vec<&str> = s.split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty()).collect();
    let mut out: Vec<String> = vec![];
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if t.eq_ignore_ascii_case("ALL") && toks.len() == 1 {
            return Ok(ds.vars.iter().map(|v| v.name.clone()).collect());
        }
        if t.eq_ignore_ascii_case("TO") {
            let prev = out.last().cloned().ok_or("TO needs a variable before it")?;
            let next = toks.get(i + 1).ok_or("TO needs a variable after it")?;
            let a = ds.var_index(&prev).ok_or(format!("Unknown variable {}", prev))?;
            let b = ds.var_index(next).ok_or(format!("Unknown variable {}", next))?;
            if b < a {
                return Err(format!("{} comes before {} in the file", next, prev));
            }
            for k in a + 1..=b {
                out.push(ds.vars[k].name.clone());
            }
            i += 2;
            continue;
        }
        match ds.var_index(t) {
            Some(k) => out.push(ds.vars[k].name.clone()),
            None => return Err(format!("Unknown variable '{}'", t)),
        }
        i += 1;
    }
    Ok(out)
}

fn one_var(s: &str, ds: &Dataset) -> Result<String, String> {
    let v = var_list(s, ds)?;
    v.into_iter().next().ok_or_else(|| "A variable is required".to_string())
}

/// (name, value) of each "/" subcommand; the first entry is the command head.
fn subcommands(body: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut depth = 0;
    let mut cur = String::new();
    let mut parts = vec![];
    for c in body.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c == '/' && depth == 0 {
            parts.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    parts.push(cur);
    for p in parts {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        let (name, val) = match p.find(|c: char| c == '=' || c.is_whitespace()) {
            Some(i) => {
                let n = p[..i].trim();
                let rest = p[i..].trim_start();
                let rest = rest.strip_prefix('=').unwrap_or(rest).trim();
                (n.to_ascii_uppercase(), rest.to_string())
            }
            None => (p.to_ascii_uppercase(), String::new()),
        };
        out.push((name, val));
    }
    out
}

fn words_upper(s: &str) -> Vec<String> {
    s.split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty()).map(|t| t.to_ascii_uppercase()).collect()
}

/// "g(1 2)" -> ("g", "1", "2")
fn group_spec(s: &str, ds: &Dataset) -> Result<(String, String, String), String> {
    let open = s.find('(').ok_or("Group values are needed, for example g(1 2)")?;
    let close = s.rfind(')').ok_or("Missing ')'")?;
    let name = one_var(&s[..open], ds)?;
    let inner = &s[open + 1..close];
    let vals: Vec<String> = inner
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .map(|t| t.trim_matches(|c| c == '\'' || c == '"').to_string())
        .collect();
    if vals.len() < 2 {
        return Err("Two group values are needed, for example g(1 2)".into());
    }
    Ok((name, vals[0].clone(), vals[1].clone()))
}

fn split_by<'a>(s: &'a str, kw: &str) -> (&'a str, Option<&'a str>) {
    let up = s.to_ascii_uppercase();
    let bytes = up.as_bytes();
    let k = kw.len();
    let mut i = 0;
    while i + k <= bytes.len() {
        if &bytes[i..i + k] == kw.as_bytes() && (i == 0 || bytes[i - 1].is_ascii_whitespace()) && (i + k == bytes.len() || bytes[i + k].is_ascii_whitespace()) {
            return (s[..i].trim(), Some(s[i + k..].trim()));
        }
        i += 1;
    }
    (s.trim(), None)
}

pub fn parse_command(cmd: &str, ds: &Dataset) -> Result<Option<Command>, String> {
    let t = cmd.trim();
    if t.is_empty() || t.starts_with('*') || t.to_ascii_uppercase().starts_with("COMMENT") {
        return Ok(None);
    }
    let up = t.to_ascii_uppercase();
    let first: String = up.split_whitespace().next().unwrap_or("").to_string();
    let after = |n: usize| t[n.min(t.len())..].trim().to_string();
    match first.as_str() {
        "EXECUTE" | "EXE" | "DATASET" | "SET" | "OUTPUT" => return Ok(None),
        "COMPUTE" => {
            let rest = after(7);
            let eq = rest.find('=').ok_or("COMPUTE needs 'target = expression'")?;
            let target = rest[..eq].trim().to_string();
            let expr = rest[eq + 1..].trim().to_string();
            crate::expr::parse(&expr, ds)?;
            return Ok(Some(Command::Compute { target, expr }));
        }
        "FILTER" | "SELECT" => {
            if up.starts_with("FILTER OFF") {
                return Ok(Some(Command::Filter { expr: None }));
            }
            let pos = up.find(" IF ").ok_or("Use FILTER IF condition, or FILTER OFF")?;
            let expr = t[pos + 4..].trim().to_string();
            crate::expr::parse(&expr, ds)?;
            return Ok(Some(Command::Filter { expr: Some(expr) }));
        }
        "USE" => return Ok(Some(Command::Filter { expr: None })),
        "SORT" => {
            let pos = up.find(" BY ").ok_or("SORT CASES BY var(A)")?;
            let spec = &t[pos + 4..];
            let mut keys = vec![];
            let mut asc = true;
            let mut pending: Vec<String> = vec![];
            let cleaned = spec.replace('(', " ( ").replace(')', " ) ");
            let mut it = cleaned.split_whitespace().peekable();
            while let Some(w) = it.next() {
                if w == "(" {
                    let d = it.next().unwrap_or("A");
                    asc = !d.eq_ignore_ascii_case("D");
                    let _ = it.next();
                    for p in pending.drain(..) {
                        keys.push((p, asc));
                    }
                } else {
                    let v = one_var(w, ds)?;
                    pending.push(v);
                }
            }
            for p in pending.drain(..) {
                keys.push((p, asc));
            }
            if keys.is_empty() {
                return Err("SORT CASES needs at least one variable".into());
            }
            return Ok(Some(Command::Sort { keys }));
        }
        _ => {}
    }
    let subs = subcommands(t);
    let get = |name: &str| subs.iter().skip(1).find(|(n, _)| n == name || n.starts_with(&format!("{}(", name))).map(|(_, v)| v.clone());
    let has = |name: &str| subs.iter().skip(1).any(|(n, _)| n == name || n.starts_with(&format!("{}(", name)));
    let head = &subs[0];
    let head_rest = head.1.clone();
    let head_val = |key: &str| -> Option<String> {
        let u = head_rest.to_ascii_uppercase();
        let k = format!("{}", key);
        if let Some(stripped) = u.strip_prefix(&k) {
            let off = head_rest.len() - stripped.len();
            let r = head_rest[off..].trim_start();
            Some(r.strip_prefix('=').unwrap_or(r).trim().to_string())
        } else {
            None
        }
    };
    match first.as_str() {
        "DESCRIPTIVES" | "DESC" => {
            let v = head_val("VARIABLES").or_else(|| get("VARIABLES")).unwrap_or_else(|| head_rest.clone());
            let vars = var_list(&v, ds)?;
            let mut stats = get("STATISTICS").map(|s| words_upper(&s)).unwrap_or_else(|| vec!["MEAN".into(), "STDDEV".into(), "MIN".into(), "MAX".into()]);
            if stats.iter().any(|s| s == "DEFAULT") {
                stats = vec!["MEAN".into(), "STDDEV".into(), "MIN".into(), "MAX".into()];
            }
            Ok(Some(Command::Descriptives { vars, stats, save: has("SAVE") }))
        }
        "FREQUENCIES" | "FREQ" => {
            let v = head_val("VARIABLES").or_else(|| get("VARIABLES")).unwrap_or_else(|| head_rest.clone());
            let vars = var_list(&v, ds)?;
            let table = !get("FORMAT").map(|f| f.to_ascii_uppercase().contains("NOTABLE")).unwrap_or(false);
            let stats = get("STATISTICS").map(|s| words_upper(&s)).unwrap_or_default();
            let (chart, normal) = if has("BARCHART") {
                ("BAR", false)
            } else if has("PIECHART") {
                ("PIE", false)
            } else if has("HISTOGRAM") {
                ("HISTOGRAM", get("HISTOGRAM").map(|h| h.to_ascii_uppercase().contains("NORMAL")).unwrap_or(false))
            } else {
                ("", false)
            };
            Ok(Some(Command::Frequencies { vars, table, stats, chart: chart.into(), normal }))
        }
        "CROSSTABS" => {
            let spec = get("TABLES").or_else(|| head_val("TABLES")).ok_or("CROSSTABS needs /TABLES=row BY column")?;
            let (a, b) = split_by(&spec, "BY");
            let rows = var_list(a, ds)?;
            let cols = var_list(b.ok_or("CROSSTABS needs BY")?, ds)?;
            let st = get("STATISTICS").map(|s| words_upper(&s)).unwrap_or_default();
            let cells = get("CELLS").map(|s| words_upper(&s)).unwrap_or_else(|| vec!["COUNT".into()]);
            Ok(Some(Command::Crosstabs { rows, cols, cells, chisq: st.iter().any(|s| s == "CHISQ"), phi: st.iter().any(|s| s == "PHI") }))
        }
        "MEANS" => {
            let spec = head_val("TABLES").or_else(|| get("TABLES")).unwrap_or_else(|| head_rest.clone());
            let (a, b) = split_by(&spec, "BY");
            Ok(Some(Command::Means { deps: var_list(a, ds)?, by: one_var(b.ok_or("MEANS needs BY")?, ds)? }))
        }
        "T-TEST" => {
            if let Some(tv) = get("TESTVAL") {
                let value: f64 = tv.trim().parse().map_err(|_| "TESTVAL must be a number")?;
                let vars = var_list(&get("VARIABLES").ok_or("T-TEST needs /VARIABLES")?, ds)?;
                return Ok(Some(Command::TTestOne { vars, value }));
            }
            if let Some(g) = head_val("GROUPS").or_else(|| get("GROUPS")) {
                let (group, g1, g2) = group_spec(&g, ds)?;
                let vars = var_list(&get("VARIABLES").ok_or("T-TEST needs /VARIABLES")?, ds)?;
                return Ok(Some(Command::TTestInd { vars, group, g1, g2 }));
            }
            if let Some(p) = head_val("PAIRS").or_else(|| get("PAIRS")) {
                let p = p.to_string();
                let cleaned = {
                    let u = p.to_ascii_uppercase();
                    match u.find("(PAIRED)") {
                        Some(i) => p[..i].to_string(),
                        None => p.clone(),
                    }
                };
                let (a, b) = split_by(&cleaned, "WITH");
                let a = var_list(a, ds)?;
                let b = var_list(b.ok_or("PAIRS needs WITH")?, ds)?;
                if a.len() != b.len() {
                    return Err("PAIRS needs the same number of variables on each side of WITH".into());
                }
                return Ok(Some(Command::TTestPaired { pairs: a.into_iter().zip(b).collect() }));
            }
            Err("T-TEST needs TESTVAL, GROUPS or PAIRS".into())
        }
        "ONEWAY" => {
            let (a, b) = split_by(&head_rest, "BY");
            let st = get("STATISTICS").map(|s| words_upper(&s)).unwrap_or_default();
            let ph = get("POSTHOC").map(|s| s.to_ascii_uppercase()).unwrap_or_default();
            Ok(Some(Command::Oneway {
                deps: var_list(a, ds)?,
                factor: one_var(b.ok_or("ONEWAY needs BY")?, ds)?,
                desc: st.iter().any(|s| s.starts_with("DESC")),
                homog: st.iter().any(|s| s.starts_with("HOMOG")),
                bonf: ph.contains("BONFERRONI"),
            }))
        }
        "CORRELATIONS" | "CORR" => {
            let v = get("VARIABLES").or_else(|| head_val("VARIABLES")).unwrap_or_else(|| head_rest.clone());
            let pr = get("PRINT").map(|s| s.to_ascii_uppercase()).unwrap_or_default();
            Ok(Some(Command::Correlations { vars: var_list(&v, ds)?, spearman: false, flag: !pr.contains("NOSIG") }))
        }
        "NONPAR" => {
            let v = get("VARIABLES").ok_or("NONPAR CORR needs /VARIABLES")?;
            let pr = get("PRINT").map(|s| s.to_ascii_uppercase()).unwrap_or_default();
            Ok(Some(Command::Correlations { vars: var_list(&v, ds)?, spearman: true, flag: !pr.contains("NOSIG") }))
        }
        "REGRESSION" => {
            let dep = one_var(&get("DEPENDENT").ok_or("REGRESSION needs /DEPENDENT")?, ds)?;
            let m = get("METHOD").ok_or("REGRESSION needs /METHOD=ENTER vars")?;
            let mu = m.to_ascii_uppercase();
            let m = if mu.starts_with("ENTER") { m[5..].to_string() } else { m };
            Ok(Some(Command::Regression { dep, ind: var_list(&m, ds)?, desc: has("DESCRIPTIVES") }))
        }
        "RELIABILITY" => {
            let v = get("VARIABLES").ok_or("RELIABILITY needs /VARIABLES")?;
            Ok(Some(Command::Reliability { vars: var_list(&v, ds)?, item_total: get("SUMMARY").map(|s| s.to_ascii_uppercase().contains("TOTAL")).unwrap_or(false) }))
        }
        "NPAR" => {
            if let Some(m) = get("M-W") {
                let (a, b) = split_by(&m, "BY");
                let (group, g1, g2) = group_spec(b.ok_or("M-W needs BY group(1 2)")?, ds)?;
                return Ok(Some(Command::MannWhitney { vars: var_list(a, ds)?, group, g1, g2 }));
            }
            if let Some(c) = get("CHISQUARE") {
                return Ok(Some(Command::ChiSquare { vars: var_list(&c, ds)? }));
            }
            Err("NPAR TESTS supports /M-W and /CHISQUARE".into())
        }
        "GRAPH" => {
            for (n, v) in subs.iter().skip(1) {
                if n.starts_with("HISTOGRAM") {
                    return Ok(Some(Command::Histogram { var: one_var(v, ds)?, normal: n.contains("NORMAL") }));
                }
                if n.starts_with("BAR") {
                    let (a, b) = split_by(v, "BY");
                    let cat = one_var(b.ok_or("BAR needs BY")?, ds)?;
                    let au = a.to_ascii_uppercase();
                    let mean_of = if au.starts_with("MEAN(") {
                        let inner = &a[5..a.rfind(')').unwrap_or(a.len())];
                        Some(one_var(inner, ds)?)
                    } else {
                        None
                    };
                    return Ok(Some(Command::Bar { cat, mean_of }));
                }
                if n.starts_with("SCATTERPLOT") {
                    let (a, b) = split_by(v, "WITH");
                    return Ok(Some(Command::Scatter { x: one_var(a, ds)?, y: one_var(b.ok_or("SCATTERPLOT needs WITH")?, ds)?, fit: has("FITLINE") }));
                }
                if n.starts_with("PIE") {
                    let (_, b) = split_by(v, "BY");
                    return Ok(Some(Command::Pie { cat: one_var(b.ok_or("PIE needs BY")?, ds)? }));
                }
            }
            Err("GRAPH supports /HISTOGRAM, /BAR, /SCATTERPLOT and /PIE".into())
        }
        "EXAMINE" => {
            let v = head_val("VARIABLES").or_else(|| get("VARIABLES")).ok_or("EXAMINE needs VARIABLES=")?;
            let (a, b) = split_by(&v, "BY");
            Ok(Some(Command::Boxplot { var: one_var(a, ds)?, by: match b {
                Some(b) => Some(one_var(b, ds)?),
                None => None,
            } }))
        }
        "DISPLAY" => {
            let v = get("VARIABLES").map(|s| var_list(&s, ds)).transpose()?.unwrap_or_else(|| ds.vars.iter().map(|v| v.name.clone()).collect());
            Ok(Some(Command::Display { vars: v }))
        }
        _ => Err(format!("'{}' isn't a command this program knows.", first)),
    }
}

pub const SYNTAX_HELP: &str = "Commands end with a period. Supported: DESCRIPTIVES, FREQUENCIES, CROSSTABS, MEANS, T-TEST, ONEWAY, CORRELATIONS, NONPAR CORR, REGRESSION, RELIABILITY, NPAR TESTS, GRAPH, EXAMINE, COMPUTE, SORT CASES, FILTER IF / FILTER OFF, DISPLAY DICTIONARY.";
