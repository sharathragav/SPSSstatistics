//! Statistical procedures. Each returns output parts (tables and charts).

use crate::data::{norm_key, Dataset};
use crate::output::{sv, BoxStat, Chart, ChartKind, Part, Table};
use std::f64::consts::PI;

// ---------------------------------------------------------------- distributions

pub fn ln_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        (PI / (PI * x).sin()).ln() - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut a = C[0];
        let t = x + G + 0.5;
        for (i, c) in C.iter().enumerate().skip(1) {
            a += c / (x + i as f64);
        }
        0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
    }
}

const FPMIN: f64 = 1e-300;

fn betacf(a: f64, b: f64, x: f64) -> f64 {
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..400 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 3e-15 {
            break;
        }
    }
    h
}

/// Regularized incomplete beta I_x(a, b).
pub fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Regularized lower incomplete gamma P(a, x).
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        let mut ap = a;
        let mut sum = 1.0 / a;
        let mut del = sum;
        for _ in 0..1000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-15 {
                break;
            }
        }
        sum * (-x + a * x.ln() - ln_gamma(a)).exp()
    } else {
        1.0 - gamma_q_cf(a, x)
    }
}

fn gamma_q_cf(a: f64, x: f64) -> f64 {
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..1000 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = b + an / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

pub fn gamma_q(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        1.0 - gamma_p(a, x)
    } else {
        gamma_q_cf(a, x)
    }
}

/// Two-tailed p-value of Student's t.
pub fn t_sig2(t: f64, df: f64) -> f64 {
    if !t.is_finite() || df <= 0.0 {
        return if t.is_infinite() { 0.0 } else { f64::NAN };
    }
    inc_beta(df / 2.0, 0.5, df / (df + t * t)).clamp(0.0, 1.0)
}
/// Upper-tail p of F.
pub fn f_sig(f: f64, d1: f64, d2: f64) -> f64 {
    if !f.is_finite() {
        return if f.is_infinite() { 0.0 } else { f64::NAN };
    }
    if f <= 0.0 {
        return 1.0;
    }
    inc_beta(d2 / 2.0, d1 / 2.0, d2 / (d2 + d1 * f)).clamp(0.0, 1.0)
}
pub fn chi2_sig(x: f64, df: f64) -> f64 {
    if !x.is_finite() || df <= 0.0 {
        return f64::NAN;
    }
    gamma_q(df / 2.0, x / 2.0).clamp(0.0, 1.0)
}
pub fn norm_cdf(z: f64) -> f64 {
    let e = gamma_p(0.5, z * z / 2.0);
    if z >= 0.0 {
        0.5 + 0.5 * e
    } else {
        0.5 - 0.5 * e
    }
}
/// Critical t for a two-tailed alpha.
pub fn t_crit(alpha: f64, df: f64) -> f64 {
    if df <= 0.0 {
        return f64::NAN;
    }
    let (mut lo, mut hi) = (0.0, 1000.0);
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if t_sig2(mid, df) > alpha {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

// ---------------------------------------------------------------- formatting

/// Number with `d` decimals, leading zero dropped for |v| < 1 (".123").
pub fn fnum(v: f64, d: usize) -> String {
    if !v.is_finite() {
        return ".".into();
    }
    let s = format!("{:.*}", d, v);
    let s = if let Some(rest) = s.strip_prefix("0.") {
        format!(".{}", rest)
    } else if let Some(rest) = s.strip_prefix("-0.") {
        format!("-.{}", rest)
    } else {
        s
    };
    if s.starts_with("-.") && s[2..].chars().all(|c| c == '0') {
        s[1..].to_string()
    } else {
        s
    }
}
pub fn fsig(p: f64) -> String {
    fnum(p, 3)
}
pub fn fpct(p: f64) -> String {
    if !p.is_finite() {
        ".".into()
    } else {
        format!("{:.1}", p)
    }
}
pub fn fpct_s(p: f64) -> String {
    if !p.is_finite() {
        ".".into()
    } else {
        format!("{:.1}%", p)
    }
}

// ---------------------------------------------------------------- basics

pub fn mean(x: &[f64]) -> f64 {
    if x.is_empty() {
        f64::NAN
    } else {
        x.iter().sum::<f64>() / x.len() as f64
    }
}
pub fn variance(x: &[f64]) -> f64 {
    if x.len() < 2 {
        return f64::NAN;
    }
    let m = mean(x);
    x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (x.len() - 1) as f64
}
pub fn sd(x: &[f64]) -> f64 {
    variance(x).sqrt()
}
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return f64::NAN;
    }
    let (mx, my) = (mean(x), mean(y));
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for i in 0..n {
        sxy += (x[i] - mx) * (y[i] - my);
        sxx += (x[i] - mx).powi(2);
        syy += (y[i] - my).powi(2);
    }
    if sxx == 0.0 || syy == 0.0 {
        f64::NAN
    } else {
        sxy / (sxx * syy).sqrt()
    }
}
fn corr_sig(r: f64, n: usize) -> f64 {
    if n < 3 || !r.is_finite() {
        return f64::NAN;
    }
    if r.abs() >= 1.0 {
        return 0.0;
    }
    let t = r * ((n as f64 - 2.0) / (1.0 - r * r)).sqrt();
    t_sig2(t, n as f64 - 2.0)
}
/// Percentile with the weighted average definition at (n+1)p.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    let w = (n as f64 + 1.0) * p;
    if w <= 1.0 {
        return sorted[0];
    }
    if w >= n as f64 {
        return sorted[n - 1];
    }
    let i = w.floor() as usize;
    let f = w - i as f64;
    sorted[i - 1] + f * (sorted[i] - sorted[i - 1])
}
fn sorted(x: &[f64]) -> Vec<f64> {
    let mut s = x.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    s
}
fn skew_kurt(x: &[f64]) -> (f64, f64, f64, f64) {
    let n = x.len() as f64;
    let m = mean(x);
    let s = sd(x);
    let (mut s3, mut s4) = (0.0, 0.0);
    for v in x {
        let z = (v - m) / s;
        s3 += z.powi(3);
        s4 += z.powi(4);
    }
    let skew = if n > 2.0 && s > 0.0 { n / ((n - 1.0) * (n - 2.0)) * s3 } else { f64::NAN };
    let kurt = if n > 3.0 && s > 0.0 { n * (n + 1.0) / ((n - 1.0) * (n - 2.0) * (n - 3.0)) * s4 - 3.0 * (n - 1.0).powi(2) / ((n - 2.0) * (n - 3.0)) } else { f64::NAN };
    let se_s = if n > 2.0 { (6.0 * n * (n - 1.0) / ((n - 2.0) * (n + 1.0) * (n + 3.0))).sqrt() } else { f64::NAN };
    let se_k = if n > 3.0 { 2.0 * se_s * ((n * n - 1.0) / ((n - 3.0) * (n + 5.0))).sqrt() } else { f64::NAN };
    (skew, se_s, kurt, se_k)
}
/// Ranks with ties averaged; also returns sum of (t^3 - t) over tie groups.
fn ranks(x: &[f64]) -> (Vec<f64>, f64) {
    let n = x.len();
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&a, &b| x[a].partial_cmp(&x[b]).unwrap_or(std::cmp::Ordering::Equal));
    let mut r = vec![0.0; n];
    let mut ties = 0.0;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && x[idx[j + 1]] == x[idx[i]] {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            r[idx[k]] = avg;
        }
        let t = (j - i + 1) as f64;
        ties += t * t * t - t;
        i = j + 1;
    }
    (r, ties)
}

fn vals(ds: &Dataset, vi: usize) -> Vec<f64> {
    let v = &ds.vars[vi];
    ds.rows().filter_map(|r| v.valid_num(r)).collect()
}
fn listwise_rows(ds: &Dataset, vis: &[usize]) -> Vec<usize> {
    ds.rows().filter(|&r| vis.iter().all(|&vi| ds.vars[vi].valid_num(r).is_some())).collect()
}
fn dec(ds: &Dataset, vi: usize) -> usize {
    (ds.vars[vi].decimals + 2).min(5)
}
fn need_numeric(ds: &Dataset, vis: &[usize]) -> Result<(), String> {
    for &vi in vis {
        if !ds.vars[vi].is_numeric() {
            return Err(format!("{} is a string variable; this procedure needs numeric variables.", ds.vars[vi].name));
        }
    }
    Ok(())
}
/// Valid, included cases grouped by the value of variable `gi`.
fn groups(ds: &Dataset, gi: usize) -> Vec<(String, Vec<usize>)> {
    let g = &ds.vars[gi];
    let cats = ds.categories(gi);
    let mut out: Vec<(String, Vec<usize>)> = cats.into_iter().map(|k| (k, vec![])).collect();
    for r in ds.rows() {
        if g.is_missing(r) {
            continue;
        }
        let k = g.key(r);
        if let Some(e) = out.iter_mut().find(|(kk, _)| *kk == k) {
            e.1.push(r);
        }
    }
    out
}

// ---------------------------------------------------------------- Descriptives

pub fn descriptives(ds: &mut Dataset, vars: &[usize], stats: &[String], save_z: bool) -> Result<Vec<Part>, String> {
    need_numeric(ds, vars)?;
    let has = |s: &str| stats.iter().any(|x| x.eq_ignore_ascii_case(s));
    let mut heads = vec!["N"];
    let order = ["RANGE", "MIN", "MAX", "SUM", "MEAN", "SEMEAN", "STDDEV", "VARIANCE", "SKEWNESS", "KURTOSIS"];
    let names = ["Range", "Minimum", "Maximum", "Sum", "Mean", "Std. Error of Mean", "Std. Deviation", "Variance", "Skewness", "Kurtosis"];
    let mut chosen = vec![];
    for (k, nm) in order.iter().zip(names.iter()) {
        if has(k) {
            chosen.push(*k);
            heads.push(nm);
            if *k == "SKEWNESS" {
                heads.push("Std. Error of Skewness");
            }
            if *k == "KURTOSIS" {
                heads.push("Std. Error of Kurtosis");
            }
        }
    }
    let mut t = Table::new("Descriptive Statistics", 1, &heads);
    for &vi in vars {
        let x = vals(ds, vi);
        let s = sorted(&x);
        let d = ds.vars[vi].decimals;
        let dd = dec(ds, vi);
        let mut cells = vec![x.len().to_string()];
        let (sk, ses, ku, sek) = skew_kurt(&x);
        let first = s.first().copied().unwrap_or(f64::NAN);
        let last = s.last().copied().unwrap_or(f64::NAN);
        for k in &chosen {
            match *k {
                "RANGE" => cells.push(fnum(last - first, d)),
                "MIN" => cells.push(fnum(first, d)),
                "MAX" => cells.push(fnum(last, d)),
                "SUM" => cells.push(fnum(x.iter().sum(), d)),
                "MEAN" => cells.push(fnum(mean(&x), dd)),
                "SEMEAN" => cells.push(fnum(sd(&x) / (x.len() as f64).sqrt(), dd + 1)),
                "STDDEV" => cells.push(fnum(sd(&x), dd + 1)),
                "VARIANCE" => cells.push(fnum(variance(&x), dd)),
                "SKEWNESS" => {
                    cells.push(fnum(sk, 3));
                    cells.push(fnum(ses, 3));
                }
                "KURTOSIS" => {
                    cells.push(fnum(ku, 3));
                    cells.push(fnum(sek, 3));
                }
                _ => {}
            }
        }
        t.row(vec![ds.vars[vi].display_name()], cells);
    }
    let lw = listwise_rows(ds, vars).len();
    let mut cells = vec![lw.to_string()];
    cells.resize(heads.len(), String::new());
    t.row(sv(&["Valid N (listwise)"]), cells);
    let mut parts = vec![Part::Table(t)];
    if save_z {
        let mut made = vec![];
        for &vi in vars {
            let x = vals(ds, vi);
            let (m, s) = (mean(&x), sd(&x));
            if !(s > 0.0) {
                continue;
            }
            let base = format!("Z{}", ds.vars[vi].name);
            let name = ds.unique_name(&base, None);
            let mut nv = crate::data::Variable::new_numeric(&name, ds.ncases);
            nv.decimals = 5;
            nv.columns = 11;
            nv.label = format!("Zscore: {}", ds.vars[vi].title());
            for r in 0..ds.ncases {
                if ds.included(r) {
                    if let Some(v) = ds.vars[vi].valid_num(r) {
                        nv.set_num(r, (v - m) / s);
                    }
                }
            }
            ds.add_var(nv);
            made.push(name);
        }
        if !made.is_empty() {
            parts.push(Part::Text(format!("Standardized values were saved as new variables: {}.", made.join(", "))));
        }
    }
    Ok(parts)
}

// ---------------------------------------------------------------- Frequencies

fn mode_of(s: &[f64]) -> (f64, bool) {
    let mut best = (f64::NAN, 0usize);
    let mut ties = 0;
    let mut i = 0;
    while i < s.len() {
        let mut j = i;
        while j < s.len() && s[j] == s[i] {
            j += 1;
        }
        let c = j - i;
        if c > best.1 {
            best = (s[i], c);
            ties = 1;
        } else if c == best.1 {
            ties += 1;
        }
        i = j;
    }
    (best.0, ties > 1)
}

pub fn frequencies(ds: &Dataset, vars: &[usize], show_table: bool, stats: &[String], chart: &str, normal: bool) -> Result<Vec<Part>, String> {
    let has = |s: &str| stats.iter().any(|x| x.eq_ignore_ascii_case(s));
    let mut parts = vec![];
    let total = ds.rows().count();
    let mut st = Table::new("Statistics", 2, &[]);
    st.headers = vars.iter().map(|&vi| ds.vars[vi].display_name()).collect();
    let list: Vec<(&str, &str)> = vec![
        ("MEAN", "Mean"),
        ("SEMEAN", "Std. Error of Mean"),
        ("MEDIAN", "Median"),
        ("MODE", "Mode"),
        ("STDDEV", "Std. Deviation"),
        ("VARIANCE", "Variance"),
        ("SKEWNESS", "Skewness"),
        ("KURTOSIS", "Kurtosis"),
        ("RANGE", "Range"),
        ("MINIMUM", "Minimum"),
        ("MAXIMUM", "Maximum"),
        ("SUM", "Sum"),
    ];
    let chosen: Vec<(&str, &str)> = list.iter().copied().filter(|(k, _)| has(k)).collect();
    let quart = has("QUARTILES");
    let mut srows: Vec<(Vec<String>, Vec<String>)> = chosen.iter().map(|(_, name)| (vec![name.to_string(), String::new()], vec![])).collect();
    if quart {
        srows.push((sv(&["Percentiles", "25"]), vec![]));
        srows.push((sv(&["", "50"]), vec![]));
        srows.push((sv(&["", "75"]), vec![]));
    }
    let mut valid_row = vec![];
    let mut miss_row = vec![];
    let mut multi_mode = false;
    for &vi in vars {
        let v = &ds.vars[vi];
        let x = if v.is_numeric() { vals(ds, vi) } else { vec![] };
        let nvalid = ds.rows().filter(|&r| !v.is_missing(r)).count();
        valid_row.push(nvalid.to_string());
        miss_row.push((total - nvalid).to_string());
        let s = sorted(&x);
        let d = v.decimals;
        let dd = dec(ds, vi);
        let (sk, _, ku, _) = skew_kurt(&x);
        let first = s.first().copied().unwrap_or(f64::NAN);
        let last = s.last().copied().unwrap_or(f64::NAN);
        for (ri, (k, _)) in chosen.iter().enumerate() {
            let cell = if !v.is_numeric() {
                String::new()
            } else {
                match *k {
                    "MEAN" => fnum(mean(&x), dd),
                    "SEMEAN" => fnum(sd(&x) / (x.len() as f64).sqrt(), dd + 1),
                    "MEDIAN" => fnum(percentile(&s, 0.5), dd),
                    "MODE" => {
                        let (m, multi) = mode_of(&s);
                        if multi {
                            multi_mode = true;
                            format!("{}a", fnum(m, d))
                        } else {
                            fnum(m, d)
                        }
                    }
                    "STDDEV" => fnum(sd(&x), dd + 1),
                    "VARIANCE" => fnum(variance(&x), dd),
                    "SKEWNESS" => fnum(sk, 3),
                    "KURTOSIS" => fnum(ku, 3),
                    "RANGE" => fnum(last - first, d),
                    "MINIMUM" => fnum(first, d),
                    "MAXIMUM" => fnum(last, d),
                    "SUM" => fnum(x.iter().sum(), d),
                    _ => String::new(),
                }
            };
            srows[ri].1.push(cell);
        }
        if quart {
            let base = chosen.len();
            for (qi, p) in [0.25, 0.5, 0.75].iter().enumerate() {
                srows[base + qi].1.push(if v.is_numeric() { fnum(percentile(&s, *p), dd) } else { String::new() });
            }
        }
    }
    st.row(sv(&["N", "Valid"]), valid_row);
    st.row(sv(&["", "Missing"]), miss_row);
    for (l, c) in srows {
        st.row(l, c);
    }
    if multi_mode {
        st.note("a. Multiple modes exist. The smallest value is shown");
    }
    parts.push(Part::Table(st));
    for &vi in vars {
        let v = &ds.vars[vi];
        let cats = ds.categories(vi);
        let mut counts = vec![0usize; cats.len()];
        let mut user_miss: Vec<(String, usize)> = vec![];
        let mut sysmis = 0usize;
        for r in ds.rows() {
            if v.is_missing(r) {
                if v.is_sysmis(r) {
                    sysmis += 1;
                } else {
                    let k = v.key(r);
                    match user_miss.iter_mut().find(|(kk, _)| *kk == k) {
                        Some(e) => e.1 += 1,
                        None => user_miss.push((k, 1)),
                    }
                }
                continue;
            }
            let k = v.key(r);
            if let Some(i) = cats.iter().position(|c| *c == k) {
                counts[i] += 1;
            }
        }
        let nvalid: usize = counts.iter().sum();
        if show_table {
            let mut t = Table::new(&v.display_name(), 2, &["Frequency", "Percent", "Valid Percent", "Cumulative Percent"]);
            let mut cum = 0.0;
            for (i, k) in cats.iter().enumerate() {
                let c = counts[i] as f64;
                cum += c / nvalid.max(1) as f64 * 100.0;
                t.row(
                    vec![if i == 0 { "Valid".into() } else { String::new() }, v.key_display(k)],
                    vec![counts[i].to_string(), fpct(c / total.max(1) as f64 * 100.0), fpct(c / nvalid.max(1) as f64 * 100.0), fpct(cum)],
                );
            }
            t.row(
                vec![if cats.is_empty() { "Valid".into() } else { String::new() }, "Total".into()],
                vec![nvalid.to_string(), fpct(nvalid as f64 / total.max(1) as f64 * 100.0), if nvalid > 0 { "100.0".into() } else { String::new() }, String::new()],
            );
            let nmiss = total - nvalid;
            if nmiss > 0 {
                let mut first = true;
                for (k, c) in &user_miss {
                    t.row(vec![if first { "Missing".into() } else { String::new() }, v.key_display(k)], vec![c.to_string(), fpct(*c as f64 / total as f64 * 100.0), String::new(), String::new()]);
                    first = false;
                }
                if sysmis > 0 {
                    t.row(vec![if first { "Missing".into() } else { String::new() }, "System".into()], vec![sysmis.to_string(), fpct(sysmis as f64 / total as f64 * 100.0), String::new(), String::new()]);
                }
                if user_miss.len() + (sysmis > 0) as usize > 1 {
                    t.row(sv(&["", "Total"]), vec![nmiss.to_string(), fpct(nmiss as f64 / total as f64 * 100.0), String::new(), String::new()]);
                }
                t.row(sv(&["Total", ""]), vec![total.to_string(), "100.0".into(), String::new(), String::new()]);
            }
            parts.push(Part::Table(t));
        }
        let labels: Vec<String> = cats.iter().map(|k| v.key_display(k)).collect();
        match chart {
            "BAR" => parts.push(Part::Chart(Chart {
                title: v.title(),
                x_label: v.title(),
                y_label: "Frequency".into(),
                kind: ChartKind::Bars { labels, values: counts.iter().map(|c| *c as f64).collect() },
            })),
            "PIE" => parts.push(Part::Chart(Chart { title: v.title(), x_label: v.title(), y_label: String::new(), kind: ChartKind::Pie { labels, values: counts.iter().map(|c| *c as f64).collect() } })),
            "HISTOGRAM" => {
                if v.is_numeric() {
                    if let Ok(mut p) = histogram(ds, vi, normal) {
                        parts.append(&mut p);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(parts)
}

// ---------------------------------------------------------------- Crosstabs

fn keys_of(ds: &Dataset, vi: usize, rows: &[usize]) -> Vec<String> {
    let v = &ds.vars[vi];
    let mut k: Vec<String> = vec![];
    for &i in rows {
        let x = v.key(i);
        if !k.contains(&x) {
            k.push(x);
        }
    }
    crate::data::sort_keys(&mut k, v.is_numeric());
    k
}

pub fn crosstabs(ds: &Dataset, rv: usize, cv: usize, cells: &[String], chisq: bool, phi: bool) -> Result<Vec<Part>, String> {
    let (r, c) = (&ds.vars[rv], &ds.vars[cv]);
    let total = ds.rows().count();
    let valid: Vec<usize> = ds.rows().filter(|&i| !r.is_missing(i) && !c.is_missing(i)).collect();
    let n = valid.len();
    let rc = keys_of(ds, rv, &valid);
    let cc = keys_of(ds, cv, &valid);
    let mut obs = vec![vec![0.0f64; cc.len()]; rc.len()];
    for &i in &valid {
        let a = rc.iter().position(|k| *k == r.key(i)).unwrap_or(0);
        let b = cc.iter().position(|k| *k == c.key(i)).unwrap_or(0);
        obs[a][b] += 1.0;
    }
    let rs: Vec<f64> = obs.iter().map(|row| row.iter().sum()).collect();
    let cs: Vec<f64> = (0..cc.len()).map(|j| obs.iter().map(|row| row[j]).sum()).collect();
    let nf = n as f64;
    let mut parts = vec![];
    let title = format!("{} * {}", r.display_name(), c.display_name());
    let mut cp = Table::new("Case Processing Summary", 1, &["N", "Percent", "N", "Percent", "N", "Percent"]);
    cp.groups = vec![("Valid".into(), 2), ("Missing".into(), 2), ("Total".into(), 2)];
    let pc = |k: usize| fpct_s(k as f64 / total.max(1) as f64 * 100.0);
    cp.row(vec![title.clone()], vec![n.to_string(), pc(n), (total - n).to_string(), pc(total - n), total.to_string(), pc(total)]);
    parts.push(Part::Table(cp));
    let want = |s: &str| cells.iter().any(|x| x.eq_ignore_ascii_case(s));
    let mut show: Vec<&str> = vec![];
    if want("COUNT") || cells.is_empty() {
        show.push("COUNT");
    }
    for s in ["EXPECTED", "ROW", "COLUMN", "TOTAL"] {
        if want(s) {
            show.push(s);
        }
    }
    let mut t = Table::new(&format!("{} Crosstabulation", title), 3, &[]);
    t.headers = cc.iter().map(|k| c.key_display(k)).collect();
    t.headers.push("Total".into());
    t.groups = vec![(c.display_name(), cc.len()), (String::new(), 1)];
    let stat_name = |s: &str| -> String {
        match s {
            "COUNT" => "Count".into(),
            "EXPECTED" => "Expected Count".into(),
            "ROW" => format!("% within {}", r.title()),
            "COLUMN" => format!("% within {}", c.title()),
            _ => "% of Total".into(),
        }
    };
    let cell_val = |s: &str, o: f64, i: Option<usize>, j: Option<usize>| -> String {
        let rsum = i.map(|i| rs[i]).unwrap_or(nf);
        let csum = j.map(|j| cs[j]).unwrap_or(nf);
        match s {
            "COUNT" => format!("{}", o as i64),
            "EXPECTED" => format!("{:.1}", rsum * csum / nf.max(1.0)),
            "ROW" => fpct_s(o / rsum.max(1e-300) * 100.0),
            "COLUMN" => fpct_s(o / csum.max(1e-300) * 100.0),
            _ => fpct_s(o / nf.max(1e-300) * 100.0),
        }
    };
    for (i, rk) in rc.iter().enumerate() {
        for (si, s) in show.iter().enumerate() {
            let l0 = if i == 0 && si == 0 { r.display_name() } else { String::new() };
            let l1 = if si == 0 { r.key_display(rk) } else { String::new() };
            let mut row: Vec<String> = (0..cc.len()).map(|j| cell_val(s, obs[i][j], Some(i), Some(j))).collect();
            row.push(cell_val(s, rs[i], Some(i), None));
            t.row(vec![l0, l1, stat_name(s)], row);
        }
    }
    for (si, s) in show.iter().enumerate() {
        let l0 = if si == 0 { "Total".to_string() } else { String::new() };
        let mut row: Vec<String> = (0..cc.len()).map(|j| cell_val(s, cs[j], None, Some(j))).collect();
        row.push(cell_val(s, nf, None, None));
        t.row(vec![l0, String::new(), stat_name(s)], row);
    }
    parts.push(Part::Table(t));
    let (nr, ncol) = (rc.len(), cc.len());
    let mut x2 = 0.0;
    let mut g2 = 0.0;
    let mut yates = 0.0;
    let mut small = 0;
    let mut min_e = f64::MAX;
    for i in 0..nr {
        for j in 0..ncol {
            let e = rs[i] * cs[j] / nf;
            if e < 5.0 {
                small += 1;
            }
            min_e = min_e.min(e);
            if e > 0.0 {
                x2 += (obs[i][j] - e).powi(2) / e;
                if obs[i][j] > 0.0 {
                    g2 += 2.0 * obs[i][j] * (obs[i][j] / e).ln();
                }
                yates += ((obs[i][j] - e).abs() - 0.5).max(0.0).powi(2) / e;
            }
        }
    }
    let df = ((nr.max(1) - 1) * (ncol.max(1) - 1)) as f64;
    if chisq {
        let mut t = Table::new("Chi-Square Tests", 1, &["Value", "df", "Asymptotic Significance (2-sided)"]);
        t.row(sv(&["Pearson Chi-Square"]), vec![format!("{}a", fnum(x2, 3)), format!("{}", df), fsig(chi2_sig(x2, df))]);
        if nr == 2 && ncol == 2 {
            t.row(sv(&["Continuity Correction"]), vec![fnum(yates, 3), "1".into(), fsig(chi2_sig(yates, 1.0))]);
        }
        t.row(sv(&["Likelihood Ratio"]), vec![fnum(g2, 3), format!("{}", df), fsig(chi2_sig(g2, df))]);
        if r.is_numeric() && c.is_numeric() && n > 2 {
            let xs: Vec<f64> = valid.iter().map(|&i| r.raw_num(i)).collect();
            let ys: Vec<f64> = valid.iter().map(|&i| c.raw_num(i)).collect();
            let rr = pearson(&xs, &ys);
            if rr.is_finite() {
                let lbl = (nf - 1.0) * rr * rr;
                t.row(sv(&["Linear-by-Linear Association"]), vec![fnum(lbl, 3), "1".into(), fsig(chi2_sig(lbl, 1.0))]);
            }
        }
        t.row(sv(&["N of Valid Cases"]), vec![n.to_string(), String::new(), String::new()]);
        let ncells = nr * ncol;
        t.note(format!(
            "a. {} cells ({:.1}%) have expected count less than 5. The minimum expected count is {:.2}.",
            small,
            small as f64 / ncells.max(1) as f64 * 100.0,
            if min_e == f64::MAX { 0.0 } else { min_e }
        ));
        parts.push(Part::Table(t));
    }
    if phi {
        let mut t = Table::new("Symmetric Measures", 2, &["Value", "Approximate Significance"]);
        let p = chi2_sig(x2, df);
        let mut phi_v = (x2 / nf).sqrt();
        if nr == 2 && ncol == 2 && obs[0][0] * obs[1][1] - obs[0][1] * obs[1][0] < 0.0 {
            phi_v = -phi_v;
        }
        let k = (nr.min(ncol) as f64 - 1.0).max(1.0);
        t.row(sv(&["Nominal by Nominal", "Phi"]), vec![fnum(phi_v, 3), fsig(p)]);
        t.row(sv(&["", "Cramer's V"]), vec![fnum((x2 / (nf * k)).sqrt(), 3), fsig(p)]);
        t.row(sv(&["", "Contingency Coefficient"]), vec![fnum((x2 / (x2 + nf)).sqrt(), 3), fsig(p)]);
        t.row(sv(&["N of Valid Cases", ""]), vec![n.to_string(), String::new()]);
        parts.push(Part::Table(t));
    }
    Ok(parts)
}

// ---------------------------------------------------------------- Means

pub fn means(ds: &Dataset, deps: &[usize], by: usize) -> Result<Vec<Part>, String> {
    need_numeric(ds, deps)?;
    let g = &ds.vars[by];
    let gs = groups(ds, by);
    let mut t = Table::new("Report", 1, &[]);
    t.corner = vec![g.display_name()];
    for &d in deps {
        t.headers.extend(sv(&["Mean", "N", "Std. Deviation"]));
        if deps.len() > 1 {
            t.groups.push((ds.vars[d].display_name(), 3));
        }
    }
    if deps.len() == 1 {
        t.title = format!("Report: {}", ds.vars[deps[0]].display_name());
    }
    let mut add = |label: String, rows: &[usize]| {
        let mut cells = vec![];
        for &d in deps {
            let x: Vec<f64> = rows.iter().filter_map(|&r| ds.vars[d].valid_num(r)).collect();
            let dd = dec(ds, d);
            cells.push(fnum(mean(&x), dd));
            cells.push(x.len().to_string());
            cells.push(fnum(sd(&x), dd + 1));
        }
        t.row(vec![label], cells);
    };
    for (k, rows) in &gs {
        add(g.key_display(k), rows);
    }
    let all: Vec<usize> = gs.iter().flat_map(|(_, r)| r.iter().copied()).collect();
    add("Total".into(), &all);
    Ok(vec![Part::Table(t)])
}

// ---------------------------------------------------------------- T tests

pub fn ttest_one(ds: &Dataset, vars: &[usize], test: f64) -> Result<Vec<Part>, String> {
    need_numeric(ds, vars)?;
    let mut s = Table::new("One-Sample Statistics", 1, &["N", "Mean", "Std. Deviation", "Std. Error Mean"]);
    let mut t = Table::new("One-Sample Test", 1, &["t", "df", "Sig. (2-tailed)", "Mean Difference", "Lower", "Upper"]);
    t.groups = vec![(format!("Test Value = {}", crate::data::fmt_key(test)), 4), ("95% Confidence Interval of the Difference".into(), 2)];
    for &vi in vars {
        let x = vals(ds, vi);
        let n = x.len() as f64;
        let (m, sdv) = (mean(&x), sd(&x));
        let se = sdv / n.sqrt();
        let dd = dec(ds, vi);
        s.row(vec![ds.vars[vi].display_name()], vec![x.len().to_string(), fnum(m, dd), fnum(sdv, dd + 1), fnum(se, dd + 1)]);
        let tv = (m - test) / se;
        let df = n - 1.0;
        let tc = t_crit(0.05, df);
        t.row(
            vec![ds.vars[vi].display_name()],
            vec![fnum(tv, 3), format!("{}", df), fsig(t_sig2(tv, df)), fnum(m - test, dd), fnum(m - test - tc * se, dd), fnum(m - test + tc * se, dd)],
        );
    }
    Ok(vec![Part::Table(s), Part::Table(t)])
}

fn group_match(ds: &Dataset, gi: usize, g: &str) -> Vec<usize> {
    let gv = &ds.vars[gi];
    let key = norm_key(g, gv.is_numeric());
    ds.rows().filter(|&r| !gv.is_missing(r) && gv.key(r) == key).collect()
}

pub fn ttest_ind(ds: &Dataset, vars: &[usize], gi: usize, g1: &str, g2: &str) -> Result<Vec<Part>, String> {
    need_numeric(ds, vars)?;
    let gv = &ds.vars[gi];
    let r1 = group_match(ds, gi, g1);
    let r2 = group_match(ds, gi, g2);
    if r1.is_empty() || r2.is_empty() {
        return Err(format!("Groups '{}' and '{}' of {} must both have cases. Use the group values (for example 1 and 2).", g1, g2, gv.name));
    }
    let k1 = gv.key_display(&norm_key(g1, gv.is_numeric()));
    let k2 = gv.key_display(&norm_key(g2, gv.is_numeric()));
    let mut gs = Table::new("Group Statistics", 2, &["N", "Mean", "Std. Deviation", "Std. Error Mean"]);
    gs.corner = vec![String::new(), gv.display_name()];
    let mut t = Table::new("Independent Samples Test", 2, &["F", "Sig.", "t", "df", "Sig. (2-tailed)", "Mean Difference", "Std. Error Difference", "Lower", "Upper"]);
    t.groups = vec![("Levene's Test for Equality of Variances".into(), 2), ("t-test for Equality of Means".into(), 5), ("95% CI of the Difference".into(), 2)];
    for &vi in vars {
        let v = &ds.vars[vi];
        let a: Vec<f64> = r1.iter().filter_map(|&r| v.valid_num(r)).collect();
        let b: Vec<f64> = r2.iter().filter_map(|&r| v.valid_num(r)).collect();
        let dd = dec(ds, vi);
        let (n1, n2) = (a.len() as f64, b.len() as f64);
        let (m1, m2) = (mean(&a), mean(&b));
        let (v1, v2) = (variance(&a), variance(&b));
        gs.row(vec![v.display_name(), k1.clone()], vec![a.len().to_string(), fnum(m1, dd), fnum(v1.sqrt(), dd + 1), fnum((v1 / n1).sqrt(), dd + 1)]);
        gs.row(vec![String::new(), k2.clone()], vec![b.len().to_string(), fnum(m2, dd), fnum(v2.sqrt(), dd + 1), fnum((v2 / n2).sqrt(), dd + 1)]);
        let za: Vec<f64> = a.iter().map(|x| (x - m1).abs()).collect();
        let zb: Vec<f64> = b.iter().map(|x| (x - m2).abs()).collect();
        let (lf, lp) = anova_f(&[za, zb]);
        let diff = m1 - m2;
        let df_eq = n1 + n2 - 2.0;
        let sp2 = ((n1 - 1.0) * v1 + (n2 - 1.0) * v2) / df_eq;
        let se_eq = (sp2 * (1.0 / n1 + 1.0 / n2)).sqrt();
        let t_eq = diff / se_eq;
        let se_w = (v1 / n1 + v2 / n2).sqrt();
        let t_w = diff / se_w;
        let df_w = (v1 / n1 + v2 / n2).powi(2) / ((v1 / n1).powi(2) / (n1 - 1.0) + (v2 / n2).powi(2) / (n2 - 1.0));
        let tc1 = t_crit(0.05, df_eq);
        let tc2 = t_crit(0.05, df_w);
        t.row(
            vec![v.display_name(), "Equal variances assumed".into()],
            vec![fnum(lf, 3), fsig(lp), fnum(t_eq, 3), format!("{}", df_eq), fsig(t_sig2(t_eq, df_eq)), fnum(diff, dd), fnum(se_eq, dd + 1), fnum(diff - tc1 * se_eq, dd), fnum(diff + tc1 * se_eq, dd)],
        );
        t.row(
            vec![String::new(), "Equal variances not assumed".into()],
            vec![String::new(), String::new(), fnum(t_w, 3), fnum(df_w, 3), fsig(t_sig2(t_w, df_w)), fnum(diff, dd), fnum(se_w, dd + 1), fnum(diff - tc2 * se_w, dd), fnum(diff + tc2 * se_w, dd)],
        );
    }
    Ok(vec![Part::Table(gs), Part::Table(t)])
}

pub fn ttest_paired(ds: &Dataset, pairs: &[(usize, usize)]) -> Result<Vec<Part>, String> {
    for (a, b) in pairs {
        need_numeric(ds, &[*a, *b])?;
    }
    let mut s = Table::new("Paired Samples Statistics", 2, &["Mean", "N", "Std. Deviation", "Std. Error Mean"]);
    let mut c = Table::new("Paired Samples Correlations", 2, &["N", "Correlation", "Sig."]);
    let mut t = Table::new("Paired Samples Test", 2, &["Mean", "Std. Deviation", "Std. Error Mean", "95% CI Lower", "95% CI Upper", "t", "df", "Sig. (2-tailed)"]);
    t.groups = vec![("Paired Differences".into(), 5), (String::new(), 3)];
    for (pi, &(a, b)) in pairs.iter().enumerate() {
        let rows = listwise_rows(ds, &[a, b]);
        let x: Vec<f64> = rows.iter().map(|&r| ds.vars[a].raw_num(r)).collect();
        let y: Vec<f64> = rows.iter().map(|&r| ds.vars[b].raw_num(r)).collect();
        let n = x.len() as f64;
        let pl = format!("Pair {}", pi + 1);
        let dd = dec(ds, a);
        for (j, (vi, z)) in [(a, &x), (b, &y)].iter().enumerate() {
            s.row(vec![if j == 0 { pl.clone() } else { String::new() }, ds.vars[*vi].display_name()], vec![fnum(mean(z), dd), z.len().to_string(), fnum(sd(z), dd + 1), fnum(sd(z) / n.sqrt(), dd + 1)]);
        }
        let r = pearson(&x, &y);
        c.row(vec![pl.clone(), format!("{} & {}", ds.vars[a].name, ds.vars[b].name)], vec![x.len().to_string(), fnum(r, 3), fsig(corr_sig(r, x.len()))]);
        let d: Vec<f64> = x.iter().zip(&y).map(|(p, q)| p - q).collect();
        let (md, sdd) = (mean(&d), sd(&d));
        let se = sdd / n.sqrt();
        let df = n - 1.0;
        let tc = t_crit(0.05, df);
        let tv = md / se;
        t.row(
            vec![pl, format!("{} - {}", ds.vars[a].name, ds.vars[b].name)],
            vec![fnum(md, dd), fnum(sdd, dd + 1), fnum(se, dd + 1), fnum(md - tc * se, dd), fnum(md + tc * se, dd), fnum(tv, 3), format!("{}", df), fsig(t_sig2(tv, df))],
        );
    }
    Ok(vec![Part::Table(s), Part::Table(c), Part::Table(t)])
}

// ---------------------------------------------------------------- ANOVA

/// One-way F for groups of values: (F, p).
fn anova_f(gs: &[Vec<f64>]) -> (f64, f64) {
    let all: Vec<f64> = gs.iter().flatten().copied().collect();
    let n = all.len() as f64;
    let k = gs.iter().filter(|g| !g.is_empty()).count() as f64;
    let gm = mean(&all);
    let ssb: f64 = gs.iter().filter(|g| !g.is_empty()).map(|g| g.len() as f64 * (mean(g) - gm).powi(2)).sum();
    let ssw: f64 = gs
        .iter()
        .map(|g| {
            let m = mean(g);
            g.iter().map(|x| (x - m).powi(2)).sum::<f64>()
        })
        .sum();
    let (d1, d2) = (k - 1.0, n - k);
    if d1 <= 0.0 || d2 <= 0.0 || ssw == 0.0 {
        return (f64::NAN, f64::NAN);
    }
    let f = (ssb / d1) / (ssw / d2);
    (f, f_sig(f, d1, d2))
}

fn desc_cells(x: &[f64], dd: usize, d: usize) -> Vec<String> {
    let n = x.len() as f64;
    let (m, s) = (mean(x), sd(x));
    let se = s / n.sqrt();
    let tc = t_crit(0.05, n - 1.0);
    let s2 = sorted(x);
    vec![
        x.len().to_string(),
        fnum(m, dd),
        fnum(s, dd + 1),
        fnum(se, dd + 1),
        fnum(m - tc * se, dd),
        fnum(m + tc * se, dd),
        fnum(s2.first().copied().unwrap_or(f64::NAN), d),
        fnum(s2.last().copied().unwrap_or(f64::NAN), d),
    ]
}

pub fn oneway(ds: &Dataset, deps: &[usize], factor: usize, desc: bool, homog: bool, bonf: bool) -> Result<Vec<Part>, String> {
    need_numeric(ds, deps)?;
    let fv = &ds.vars[factor];
    let gs = groups(ds, factor);
    if gs.len() < 2 {
        return Err(format!("{} needs at least two groups.", fv.name));
    }
    let mut parts = vec![];
    if desc {
        let mut t = Table::new("Descriptives", 2, &["N", "Mean", "Std. Deviation", "Std. Error", "Lower Bound", "Upper Bound", "Minimum", "Maximum"]);
        t.groups = vec![(String::new(), 4), ("95% Confidence Interval for Mean".into(), 2), (String::new(), 2)];
        for &d in deps {
            let dd = dec(ds, d);
            let dv = &ds.vars[d];
            let mut all = vec![];
            for (gi, (k, rows)) in gs.iter().enumerate() {
                let x: Vec<f64> = rows.iter().filter_map(|&r| dv.valid_num(r)).collect();
                all.extend(x.iter().copied());
                t.row(vec![if gi == 0 { dv.display_name() } else { String::new() }, fv.key_display(k)], desc_cells(&x, dd, dv.decimals));
            }
            t.row(vec![String::new(), "Total".into()], desc_cells(&all, dd, dv.decimals));
        }
        parts.push(Part::Table(t));
    }
    if homog {
        let mut t = Table::new("Test of Homogeneity of Variances", 1, &["Levene Statistic", "df1", "df2", "Sig."]);
        for &d in deps {
            let dv = &ds.vars[d];
            let zs: Vec<Vec<f64>> = gs
                .iter()
                .map(|(_, rows)| {
                    let x: Vec<f64> = rows.iter().filter_map(|&r| dv.valid_num(r)).collect();
                    let m = mean(&x);
                    x.iter().map(|v| (v - m).abs()).collect()
                })
                .collect();
            let n: usize = zs.iter().map(|z| z.len()).sum();
            let k = zs.iter().filter(|z| !z.is_empty()).count();
            let (f, p) = anova_f(&zs);
            t.row(vec![dv.display_name()], vec![fnum(f, 3), (k.saturating_sub(1)).to_string(), (n.saturating_sub(k)).to_string(), fsig(p)]);
        }
        parts.push(Part::Table(t));
    }
    let mut t = Table::new("ANOVA", 2, &["Sum of Squares", "df", "Mean Square", "F", "Sig."]);
    let mut mc_tables = vec![];
    for &d in deps {
        let dv = &ds.vars[d];
        let xs: Vec<Vec<f64>> = gs.iter().map(|(_, rows)| rows.iter().filter_map(|&r| dv.valid_num(r)).collect()).collect();
        let all: Vec<f64> = xs.iter().flatten().copied().collect();
        let n = all.len() as f64;
        let k = xs.iter().filter(|g| !g.is_empty()).count() as f64;
        let gm = mean(&all);
        let ssb: f64 = xs.iter().filter(|g| !g.is_empty()).map(|g| g.len() as f64 * (mean(g) - gm).powi(2)).sum();
        let sst: f64 = all.iter().map(|x| (x - gm).powi(2)).sum();
        let ssw = sst - ssb;
        let (d1, d2) = (k - 1.0, n - k);
        let msb = ssb / d1;
        let msw = ssw / d2;
        let f = msb / msw;
        t.row(vec![dv.display_name(), "Between Groups".into()], vec![fnum(ssb, 3), format!("{}", d1), fnum(msb, 3), fnum(f, 3), fsig(f_sig(f, d1, d2))]);
        t.row(vec![String::new(), "Within Groups".into()], vec![fnum(ssw, 3), format!("{}", d2), fnum(msw, 3), String::new(), String::new()]);
        t.row(vec![String::new(), "Total".into()], vec![fnum(sst, 3), format!("{}", n - 1.0), String::new(), String::new(), String::new()]);
        if bonf {
            let mut mt = Table::new("Multiple Comparisons", 2, &["Mean Difference (I-J)", "Std. Error", "Sig.", "Lower Bound", "Upper Bound"]);
            mt.corner = vec![format!("(I) {}", fv.title()), format!("(J) {}", fv.title())];
            mt.groups = vec![(String::new(), 3), ("95% Confidence Interval".into(), 2)];
            let kk = gs.len();
            let m = (kk * (kk - 1) / 2) as f64;
            let tc = t_crit(0.05 / m, d2);
            let dd = dec(ds, d);
            let mut any = false;
            for i in 0..kk {
                let first_j = (0..kk).find(|&q| q != i).unwrap_or(0);
                for j in 0..kk {
                    if i == j || xs[i].is_empty() || xs[j].is_empty() {
                        continue;
                    }
                    let diff = mean(&xs[i]) - mean(&xs[j]);
                    let se = (msw * (1.0 / xs[i].len() as f64 + 1.0 / xs[j].len() as f64)).sqrt();
                    let p = (t_sig2(diff / se, d2) * m).min(1.0);
                    let star = if p < 0.05 {
                        any = true;
                        "*"
                    } else {
                        ""
                    };
                    mt.row(
                        vec![if j == first_j { fv.key_display(&gs[i].0) } else { String::new() }, fv.key_display(&gs[j].0)],
                        vec![format!("{}{}", fnum(diff, dd), star), fnum(se, dd + 1), fsig(p), fnum(diff - tc * se, dd), fnum(diff + tc * se, dd)],
                    );
                }
            }
            mt.note(format!("Dependent Variable: {}", dv.display_name()));
            mt.note("Bonferroni");
            if any {
                mt.note("*. The mean difference is significant at the 0.05 level.");
            }
            mc_tables.push(Part::Table(mt));
        }
    }
    parts.push(Part::Table(t));
    parts.extend(mc_tables);
    Ok(parts)
}

// ---------------------------------------------------------------- Correlations

pub fn correlations(ds: &Dataset, vars: &[usize], spearman: bool, flag: bool) -> Result<Vec<Part>, String> {
    need_numeric(ds, vars)?;
    let name = if spearman { "Spearman's rho" } else { "Pearson Correlation" };
    let mut t = Table::new("Correlations", if spearman { 3 } else { 2 }, &[]);
    t.headers = vars.iter().map(|&v| ds.vars[v].display_name()).collect();
    let mut s1 = false;
    let mut s5 = false;
    for (i, &a) in vars.iter().enumerate() {
        let mut rr = vec![];
        let mut pp = vec![];
        let mut nn = vec![];
        for &b in vars.iter() {
            let rows = listwise_rows(ds, &[a, b]);
            let mut x: Vec<f64> = rows.iter().map(|&r| ds.vars[a].raw_num(r)).collect();
            let mut y: Vec<f64> = rows.iter().map(|&r| ds.vars[b].raw_num(r)).collect();
            if spearman {
                x = ranks(&x).0;
                y = ranks(&y).0;
            }
            if a == b {
                rr.push(if spearman { "1.000".into() } else { "1".into() });
                pp.push(String::new());
            } else {
                let r = pearson(&x, &y);
                let p = corr_sig(r, x.len());
                let star = if flag && p < 0.01 {
                    s1 = true;
                    "**"
                } else if flag && p < 0.05 {
                    s5 = true;
                    "*"
                } else {
                    ""
                };
                rr.push(format!("{}{}", fnum(r, 3), star));
                pp.push(fsig(p));
            }
            nn.push(x.len().to_string());
        }
        let vn = ds.vars[a].display_name();
        if spearman {
            t.row(vec![if i == 0 { name.into() } else { String::new() }, vn, "Correlation Coefficient".into()], rr);
            t.row(sv(&["", "", "Sig. (2-tailed)"]), pp);
            t.row(sv(&["", "", "N"]), nn);
        } else {
            t.row(vec![vn, name.into()], rr);
            t.row(sv(&["", "Sig. (2-tailed)"]), pp);
            t.row(sv(&["", "N"]), nn);
        }
    }
    if s1 {
        t.note("**. Correlation is significant at the 0.01 level (2-tailed).");
    }
    if s5 {
        t.note("*. Correlation is significant at the 0.05 level (2-tailed).");
    }
    Ok(vec![Part::Table(t)])
}

// ---------------------------------------------------------------- Regression

fn invert(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = m.to_vec();
    let mut inv: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect();
    for c in 0..n {
        let mut p = c;
        for r in c + 1..n {
            if a[r][c].abs() > a[p][c].abs() {
                p = r;
            }
        }
        if a[p][c].abs() < 1e-12 {
            return None;
        }
        a.swap(c, p);
        inv.swap(c, p);
        let d = a[c][c];
        for j in 0..n {
            a[c][j] /= d;
            inv[c][j] /= d;
        }
        for r in 0..n {
            if r != c {
                let f = a[r][c];
                if f != 0.0 {
                    for j in 0..n {
                        a[r][j] -= f * a[c][j];
                        inv[r][j] -= f * inv[c][j];
                    }
                }
            }
        }
    }
    Some(inv)
}

pub fn regression(ds: &Dataset, dep: usize, ind: &[usize], descriptives: bool) -> Result<Vec<Part>, String> {
    let mut all = vec![dep];
    all.extend_from_slice(ind);
    need_numeric(ds, &all)?;
    let rows = listwise_rows(ds, &all);
    let n = rows.len();
    let k = ind.len();
    if n <= k + 1 {
        return Err("There are too few complete cases for this regression.".into());
    }
    let y: Vec<f64> = rows.iter().map(|&r| ds.vars[dep].raw_num(r)).collect();
    let xs: Vec<Vec<f64>> = rows
        .iter()
        .map(|&r| {
            let mut v = vec![1.0];
            v.extend(ind.iter().map(|&i| ds.vars[i].raw_num(r)));
            v
        })
        .collect();
    let p = k + 1;
    let mut xtx = vec![vec![0.0; p]; p];
    let mut xty = vec![0.0; p];
    for (row, yy) in xs.iter().zip(&y) {
        for i in 0..p {
            xty[i] += row[i] * yy;
            for j in 0..p {
                xtx[i][j] += row[i] * row[j];
            }
        }
    }
    let inv = invert(&xtx).ok_or("The predictors are perfectly collinear (or constant); the model can't be estimated.")?;
    let b: Vec<f64> = (0..p).map(|i| (0..p).map(|j| inv[i][j] * xty[j]).sum()).collect();
    let my = mean(&y);
    let sst: f64 = y.iter().map(|v| (v - my).powi(2)).sum();
    let sse: f64 = xs
        .iter()
        .zip(&y)
        .map(|(row, yy)| {
            let pred: f64 = row.iter().zip(&b).map(|(a, c)| a * c).sum();
            (yy - pred).powi(2)
        })
        .sum();
    let ssr = sst - sse;
    let (dfr, dfe) = (k as f64, (n - k - 1) as f64);
    let mse = sse / dfe;
    let r2 = if sst > 0.0 { ssr / sst } else { f64::NAN };
    let adj = 1.0 - (1.0 - r2) * (n as f64 - 1.0) / dfe;
    let f = (ssr / dfr) / mse;
    let depn = ds.vars[dep].display_name();
    let pred_names: Vec<String> = ind.iter().map(|&i| ds.vars[i].display_name()).collect();
    let mut parts = vec![];
    if descriptives {
        let mut t = Table::new("Descriptive Statistics", 1, &["Mean", "Std. Deviation", "N"]);
        for &vi in &all {
            let x: Vec<f64> = rows.iter().map(|&r| ds.vars[vi].raw_num(r)).collect();
            let dd = dec(ds, vi);
            t.row(vec![ds.vars[vi].display_name()], vec![fnum(mean(&x), dd), fnum(sd(&x), dd + 1), n.to_string()]);
        }
        parts.push(Part::Table(t));
    }
    let mut ve = Table::new("Variables Entered/Removed", 1, &["Variables Entered", "Variables Removed", "Method"]);
    ve.corner = vec!["Model".into()];
    ve.row(sv(&["1"]), vec![pred_names.join(", "), String::new(), "Enter".into()]);
    ve.note(format!("a. Dependent Variable: {}", depn));
    ve.note("b. All requested variables entered.");
    parts.push(Part::Table(ve));
    let mut ms = Table::new("Model Summary", 1, &["R", "R Square", "Adjusted R Square", "Std. Error of the Estimate"]);
    ms.corner = vec!["Model".into()];
    ms.row(sv(&["1"]), vec![format!("{}a", fnum(r2.max(0.0).sqrt(), 3)), fnum(r2, 3), fnum(adj, 3), fnum(mse.sqrt(), 5)]);
    ms.note(format!("a. Predictors: (Constant), {}", pred_names.join(", ")));
    parts.push(Part::Table(ms));
    let mut an = Table::new("ANOVA", 2, &["Sum of Squares", "df", "Mean Square", "F", "Sig."]);
    an.corner = vec!["Model".into(), String::new()];
    an.row(sv(&["1", "Regression"]), vec![fnum(ssr, 3), format!("{}", dfr), fnum(ssr / dfr, 3), fnum(f, 3), format!("{}b", fsig(f_sig(f, dfr, dfe)))]);
    an.row(sv(&["", "Residual"]), vec![fnum(sse, 3), format!("{}", dfe), fnum(mse, 3), String::new(), String::new()]);
    an.row(sv(&["", "Total"]), vec![fnum(sst, 3), format!("{}", n - 1), String::new(), String::new(), String::new()]);
    an.note(format!("a. Dependent Variable: {}", depn));
    an.note(format!("b. Predictors: (Constant), {}", pred_names.join(", ")));
    parts.push(Part::Table(an));
    let mut co = Table::new("Coefficients", 2, &["B", "Std. Error", "Beta", "t", "Sig."]);
    co.corner = vec!["Model".into(), String::new()];
    co.groups = vec![("Unstandardized Coefficients".into(), 2), ("Standardized Coefficients".into(), 1), (String::new(), 2)];
    let sdy = sd(&y);
    for i in 0..p {
        let se = (inv[i][i] * mse).sqrt();
        let tv = b[i] / se;
        let beta = if i == 0 {
            String::new()
        } else {
            let x: Vec<f64> = xs.iter().map(|r| r[i]).collect();
            fnum(b[i] * sd(&x) / sdy, 3)
        };
        let label = if i == 0 { "(Constant)".to_string() } else { pred_names[i - 1].clone() };
        co.row(vec![if i == 0 { "1".into() } else { String::new() }, label], vec![fnum(b[i], 3), fnum(se, 3), beta, fnum(tv, 3), fsig(t_sig2(tv, dfe))]);
    }
    co.note(format!("a. Dependent Variable: {}", depn));
    parts.push(Part::Table(co));
    Ok(parts)
}

// ---------------------------------------------------------------- Reliability

pub fn reliability(ds: &Dataset, items: &[usize], item_total: bool) -> Result<Vec<Part>, String> {
    need_numeric(ds, items)?;
    if items.len() < 2 {
        return Err("Reliability needs at least two items.".into());
    }
    let total = ds.rows().count();
    let rows = listwise_rows(ds, items);
    let n = rows.len();
    let k = items.len() as f64;
    let mut cp = Table::new("Case Processing Summary", 2, &["N", "%"]);
    cp.row(sv(&["Cases", "Valid"]), vec![n.to_string(), fpct(n as f64 / total.max(1) as f64 * 100.0)]);
    cp.row(sv(&["", "Excluded"]), vec![(total - n).to_string(), fpct((total - n) as f64 / total.max(1) as f64 * 100.0)]);
    cp.row(sv(&["", "Total"]), vec![total.to_string(), "100.0".into()]);
    cp.note("Listwise deletion based on all variables in the procedure.");
    let cols: Vec<Vec<f64>> = items.iter().map(|&vi| rows.iter().map(|&r| ds.vars[vi].raw_num(r)).collect()).collect();
    let totals: Vec<f64> = (0..n).map(|i| cols.iter().map(|c| c[i]).sum()).collect();
    let alpha_of = |cs: &[&Vec<f64>]| -> f64 {
        let kk = cs.len() as f64;
        let tot: Vec<f64> = (0..n).map(|i| cs.iter().map(|c| c[i]).sum()).collect();
        let sum_var: f64 = cs.iter().map(|c| variance(c)).sum();
        kk / (kk - 1.0) * (1.0 - sum_var / variance(&tot))
    };
    let all: Vec<&Vec<f64>> = cols.iter().collect();
    let alpha = alpha_of(&all);
    let mut rs = Table::new("Reliability Statistics", 1, &["Cronbach's Alpha", "N of Items"]);
    rs.row(vec![String::new()], vec![fnum(alpha, 3), format!("{}", k)]);
    let mut parts = vec![Part::Table(cp), Part::Table(rs)];
    if item_total {
        let mut it = Table::new("Item-Total Statistics", 1, &["Scale Mean if Item Deleted", "Scale Variance if Item Deleted", "Corrected Item-Total Correlation", "Cronbach's Alpha if Item Deleted"]);
        for (j, &vi) in items.iter().enumerate() {
            let rest: Vec<f64> = (0..n).map(|i| totals[i] - cols[j][i]).collect();
            let others: Vec<&Vec<f64>> = cols.iter().enumerate().filter(|(q, _)| *q != j).map(|(_, c)| c).collect();
            let a = if others.len() >= 2 { alpha_of(&others) } else { f64::NAN };
            it.row(vec![ds.vars[vi].display_name()], vec![fnum(mean(&rest), 2), fnum(variance(&rest), 3), fnum(pearson(&cols[j], &rest), 3), fnum(a, 3)]);
        }
        parts.push(Part::Table(it));
    }
    Ok(parts)
}

// ---------------------------------------------------------------- Nonparametric

pub fn mann_whitney(ds: &Dataset, vars: &[usize], gi: usize, g1: &str, g2: &str) -> Result<Vec<Part>, String> {
    need_numeric(ds, vars)?;
    let gv = &ds.vars[gi];
    let r1 = group_match(ds, gi, g1);
    let r2 = group_match(ds, gi, g2);
    if r1.is_empty() || r2.is_empty() {
        return Err(format!("Groups '{}' and '{}' of {} must both have cases.", g1, g2, gv.name));
    }
    let k1 = gv.key_display(&norm_key(g1, gv.is_numeric()));
    let k2 = gv.key_display(&norm_key(g2, gv.is_numeric()));
    let mut rt = Table::new("Ranks", 2, &["N", "Mean Rank", "Sum of Ranks"]);
    rt.corner = vec![String::new(), gv.display_name()];
    let mut tt = Table::new("Test Statistics", 1, &[]);
    let (mut uu, mut ww, mut zz, mut pp) = (vec![], vec![], vec![], vec![]);
    for &vi in vars {
        let v = &ds.vars[vi];
        let a: Vec<f64> = r1.iter().filter_map(|&r| v.valid_num(r)).collect();
        let b: Vec<f64> = r2.iter().filter_map(|&r| v.valid_num(r)).collect();
        let mut all = a.clone();
        all.extend(b.iter().copied());
        let (rk, ties) = ranks(&all);
        let (n1, n2) = (a.len() as f64, b.len() as f64);
        let nn = n1 + n2;
        let sr1: f64 = rk[..a.len()].iter().sum();
        let sr2: f64 = rk[a.len()..].iter().sum();
        rt.row(vec![v.display_name(), k1.clone()], vec![a.len().to_string(), fnum(sr1 / n1, 2), fnum(sr1, 2)]);
        rt.row(vec![String::new(), k2.clone()], vec![b.len().to_string(), fnum(sr2 / n2, 2), fnum(sr2, 2)]);
        rt.row(sv(&["", "Total"]), vec![format!("{}", nn), String::new(), String::new()]);
        let u1 = sr1 - n1 * (n1 + 1.0) / 2.0;
        let u2 = n1 * n2 - u1;
        let u = u1.min(u2);
        let w = if u1 <= u2 { sr1 } else { sr2 };
        let sigma = (n1 * n2 / (nn * (nn - 1.0)) * ((nn.powi(3) - nn) / 12.0 - ties / 12.0)).sqrt();
        let z = (u - n1 * n2 / 2.0) / sigma;
        uu.push(fnum(u, 3));
        ww.push(fnum(w, 3));
        zz.push(fnum(z, 3));
        pp.push(fsig(2.0 * norm_cdf(-z.abs())));
    }
    tt.headers = vars.iter().map(|&v| ds.vars[v].display_name()).collect();
    tt.row(sv(&["Mann-Whitney U"]), uu);
    tt.row(sv(&["Wilcoxon W"]), ww);
    tt.row(sv(&["Z"]), zz);
    tt.row(sv(&["Asymp. Sig. (2-tailed)"]), pp);
    tt.note(format!("a. Grouping Variable: {}", gv.display_name()));
    Ok(vec![Part::Table(rt), Part::Table(tt)])
}

pub fn chisq_gof(ds: &Dataset, vars: &[usize]) -> Result<Vec<Part>, String> {
    let mut parts = vec![];
    let mut tt = Table::new("Test Statistics", 1, &[]);
    let (mut cs, mut dfs, mut ps, mut notes) = (vec![], vec![], vec![], vec![]);
    for (idx, &vi) in vars.iter().enumerate() {
        let v = &ds.vars[vi];
        let cats = ds.categories(vi);
        let mut counts = vec![0.0f64; cats.len()];
        for r in ds.rows() {
            if v.is_missing(r) {
                continue;
            }
            if let Some(i) = cats.iter().position(|c| *c == v.key(r)) {
                counts[i] += 1.0;
            }
        }
        let n: f64 = counts.iter().sum();
        let e = n / cats.len().max(1) as f64;
        let mut t = Table::new(&v.display_name(), 1, &["Observed N", "Expected N", "Residual"]);
        let mut x2 = 0.0;
        for (i, k) in cats.iter().enumerate() {
            x2 += (counts[i] - e).powi(2) / e;
            t.row(vec![v.key_display(k)], vec![format!("{}", counts[i]), fnum(e, 1), fnum(counts[i] - e, 1)]);
        }
        t.row(sv(&["Total"]), vec![format!("{}", n), String::new(), String::new()]);
        parts.push(Part::Table(t));
        let df = cats.len().max(1) as f64 - 1.0;
        let letter = (b'a' + (idx % 26) as u8) as char;
        cs.push(format!("{}{}", fnum(x2, 3), letter));
        dfs.push(format!("{}", df));
        ps.push(fsig(chi2_sig(x2, df)));
        notes.push(format!("{}. {} cells have expected frequencies less than 5. The minimum expected cell frequency is {:.1}.", letter, if e < 5.0 { cats.len() } else { 0 }, e));
    }
    tt.headers = vars.iter().map(|&v| ds.vars[v].display_name()).collect();
    tt.row(sv(&["Chi-Square"]), cs);
    tt.row(sv(&["df"]), dfs);
    tt.row(sv(&["Asymp. Sig."]), ps);
    tt.notes = notes;
    parts.push(Part::Table(tt));
    Ok(parts)
}

// ---------------------------------------------------------------- Charts

pub fn histogram(ds: &Dataset, vi: usize, normal: bool) -> Result<Vec<Part>, String> {
    need_numeric(ds, &[vi])?;
    let x = vals(ds, vi);
    if x.is_empty() {
        return Err(format!("{} has no valid values.", ds.vars[vi].name));
    }
    let s = sorted(&x);
    let (lo, hi) = (s[0], s[s.len() - 1]);
    let k = ((x.len() as f64).sqrt().round() as usize).clamp(5, 30);
    let raw = if hi > lo { (hi - lo) / k as f64 } else { 1.0 };
    let e = raw.log10().floor();
    let f = raw / 10f64.powf(e);
    let step = (if f < 1.5 {
        1.0
    } else if f < 3.0 {
        2.0
    } else if f < 7.0 {
        5.0
    } else {
        10.0
    }) * 10f64.powf(e);
    let start = (lo / step).floor() * step;
    let mut edges = vec![start];
    while edges[edges.len() - 1] <= hi && edges.len() < 200 {
        let nx = edges[edges.len() - 1] + step;
        edges.push(nx);
    }
    let mut counts = vec![0.0; edges.len() - 1];
    for v in &x {
        let i = (((v - start) / step).floor().max(0.0) as usize).min(counts.len() - 1);
        counts[i] += 1.0;
    }
    let v = &ds.vars[vi];
    Ok(vec![Part::Chart(Chart {
        title: v.title(),
        x_label: v.title(),
        y_label: "Frequency".into(),
        kind: ChartKind::Histogram { edges, counts, mean: mean(&x), sd: sd(&x), n: x.len(), normal },
    })])
}

pub fn bar_chart(ds: &Dataset, cat: usize, mean_of: Option<usize>) -> Result<Vec<Part>, String> {
    let c = &ds.vars[cat];
    let gs = groups(ds, cat);
    let labels: Vec<String> = gs.iter().map(|(k, _)| c.key_display(k)).collect();
    let (values, ylab, title) = match mean_of {
        None => (gs.iter().map(|(_, r)| r.len() as f64).collect::<Vec<_>>(), "Count".to_string(), format!("Count by {}", c.title())),
        Some(m) => {
            need_numeric(ds, &[m])?;
            let mv = &ds.vars[m];
            (
                gs.iter().map(|(_, r)| mean(&r.iter().filter_map(|&i| mv.valid_num(i)).collect::<Vec<_>>())).collect(),
                format!("Mean {}", mv.title()),
                format!("Mean of {} by {}", mv.title(), c.title()),
            )
        }
    };
    Ok(vec![Part::Chart(Chart { title, x_label: c.title(), y_label: ylab, kind: ChartKind::Bars { labels, values } })])
}

pub fn scatter(ds: &Dataset, xv: usize, yv: usize, fit: bool) -> Result<Vec<Part>, String> {
    need_numeric(ds, &[xv, yv])?;
    let rows = listwise_rows(ds, &[xv, yv]);
    let pts: Vec<(f64, f64)> = rows.iter().map(|&r| (ds.vars[xv].raw_num(r), ds.vars[yv].raw_num(r))).collect();
    let fitv = if fit && pts.len() > 2 {
        let x: Vec<f64> = pts.iter().map(|p| p.0).collect();
        let y: Vec<f64> = pts.iter().map(|p| p.1).collect();
        let r = pearson(&x, &y);
        let b1 = r * sd(&y) / sd(&x);
        let b0 = mean(&y) - b1 * mean(&x);
        if b1.is_finite() {
            Some((b0, b1, r * r))
        } else {
            None
        }
    } else {
        None
    };
    let (x, y) = (&ds.vars[xv], &ds.vars[yv]);
    Ok(vec![Part::Chart(Chart { title: format!("{} by {}", y.title(), x.title()), x_label: x.title(), y_label: y.title(), kind: ChartKind::Scatter { pts, fit: fitv } })])
}

fn box_stat(x: &[f64]) -> BoxStat {
    let s = sorted(x);
    let q1 = percentile(&s, 0.25);
    let q3 = percentile(&s, 0.75);
    let iqr = q3 - q1;
    let (lf, uf) = (q1 - 1.5 * iqr, q3 + 1.5 * iqr);
    let lo = s.iter().copied().find(|v| *v >= lf).unwrap_or(q1);
    let hi = s.iter().rev().copied().find(|v| *v <= uf).unwrap_or(q3);
    BoxStat { lo, q1, med: percentile(&s, 0.5), q3, hi, outliers: s.iter().copied().filter(|v| *v < lf || *v > uf).collect(), n: s.len() }
}

pub fn boxplot(ds: &Dataset, vi: usize, by: Option<usize>) -> Result<Vec<Part>, String> {
    need_numeric(ds, &[vi])?;
    let v = &ds.vars[vi];
    let groups_v: Vec<(String, BoxStat)> = match by {
        None => {
            let x = vals(ds, vi);
            if x.is_empty() {
                return Err(format!("{} has no valid values.", v.name));
            }
            vec![(v.title(), box_stat(&x))]
        }
        Some(g) => {
            let gv = &ds.vars[g];
            groups(ds, g)
                .into_iter()
                .filter_map(|(k, rows)| {
                    let x: Vec<f64> = rows.iter().filter_map(|&r| v.valid_num(r)).collect();
                    if x.is_empty() {
                        None
                    } else {
                        Some((gv.key_display(&k), box_stat(&x)))
                    }
                })
                .collect()
        }
    };
    let xl = by.map(|g| ds.vars[g].title()).unwrap_or_default();
    let mut t = Table::new(&format!("Descriptives: {}", v.display_name()), 1, &["N", "Median", "25th Percentile", "75th Percentile", "Minimum", "Maximum"]);
    if let Some(g) = by {
        t.corner = vec![ds.vars[g].display_name()];
    }
    let dd = dec(ds, vi);
    for (name, b) in &groups_v {
        let mn = b.outliers.iter().copied().fold(b.lo, f64::min);
        let mx = b.outliers.iter().copied().fold(b.hi, f64::max);
        t.row(vec![name.clone()], vec![b.n.to_string(), fnum(b.med, dd), fnum(b.q1, dd), fnum(b.q3, dd), fnum(mn, v.decimals), fnum(mx, v.decimals)]);
    }
    Ok(vec![Part::Table(t), Part::Chart(Chart { title: v.title(), x_label: xl, y_label: v.title(), kind: ChartKind::Boxes { groups: groups_v } })])
}

pub fn pie(ds: &Dataset, cat: usize) -> Result<Vec<Part>, String> {
    let c = &ds.vars[cat];
    let gs = groups(ds, cat);
    Ok(vec![Part::Chart(Chart {
        title: c.title(),
        x_label: c.title(),
        y_label: String::new(),
        kind: ChartKind::Pie { labels: gs.iter().map(|(k, _)| c.key_display(k)).collect(), values: gs.iter().map(|(_, r)| r.len() as f64).collect() },
    })])
}

pub fn dictionary(ds: &Dataset, vars: &[usize]) -> Vec<Part> {
    let mut t = Table::new("Variable Information", 1, &["Position", "Label", "Measurement Level", "Role", "Column Width", "Alignment", "Print Format", "Missing Values"]);
    t.corner = vec!["Variable".into()];
    for &vi in vars {
        let v = &ds.vars[vi];
        let fmt = if v.is_numeric() { format!("F{}.{}", v.width, v.decimals) } else { format!("A{}", v.width) };
        t.row(vec![v.name.clone()], vec![(vi + 1).to_string(), v.label.clone(), v.measure.name().into(), v.role.name().into(), v.columns.to_string(), v.align.name().into(), fmt, v.missing.join(", ")]);
    }
    let mut parts = vec![Part::Table(t)];
    let mut lt = Table::new("Variable Values", 2, &["Label"]);
    lt.corner = vec!["Variable".into(), "Value".into()];
    for &vi in vars {
        let v = &ds.vars[vi];
        for (i, (k, l)) in v.value_labels.iter().enumerate() {
            lt.row(vec![if i == 0 { v.name.clone() } else { String::new() }, k.clone()], vec![l.clone()]);
        }
    }
    if !lt.rows.is_empty() {
        parts.push(Part::Table(lt));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dists() {
        assert!((t_sig2(2.0, 10.0) - 0.0734).abs() < 1e-3);
        assert!((f_sig(4.0, 2.0, 20.0) - 0.0345).abs() < 1e-3);
        assert!((chi2_sig(3.841, 1.0) - 0.05).abs() < 1e-3);
        assert!((norm_cdf(1.96) - 0.975).abs() < 1e-3);
        assert!((t_crit(0.05, 10.0) - 2.228).abs() < 1e-3);
    }
}
