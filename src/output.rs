//! Output items: result tables and charts, how they are drawn and exported.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

#[derive(Clone, Debug)]
pub struct TRow {
    pub labels: Vec<String>,
    pub cells: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Table {
    pub title: String,
    pub corner: Vec<String>,
    pub groups: Vec<(String, usize)>,
    pub headers: Vec<String>,
    pub rows: Vec<TRow>,
    pub notes: Vec<String>,
}

pub fn sv<S: AsRef<str>>(a: &[S]) -> Vec<String> {
    a.iter().map(|s| s.as_ref().to_string()).collect()
}

impl Table {
    pub fn new(title: &str, levels: usize, headers: &[&str]) -> Self {
        Table { title: title.into(), corner: vec![String::new(); levels.max(1)], headers: sv(headers), ..Default::default() }
    }
    pub fn row(&mut self, labels: Vec<String>, cells: Vec<String>) {
        self.rows.push(TRow { labels, cells });
    }
    pub fn note(&mut self, s: impl Into<String>) {
        self.notes.push(s.into());
    }
    pub fn to_tsv(&self) -> String {
        let mut o = format!("{}\n", self.title);
        if !self.groups.is_empty() {
            let mut line: Vec<String> = vec![String::new(); self.corner.len()];
            for (g, n) in &self.groups {
                line.push(g.clone());
                for _ in 1..*n {
                    line.push(String::new());
                }
            }
            o.push_str(&line.join("\t"));
            o.push('\n');
        }
        let mut head = self.corner.clone();
        head.extend(self.headers.iter().cloned());
        o.push_str(&head.join("\t"));
        o.push('\n');
        for r in &self.rows {
            let mut l = r.labels.clone();
            l.resize(self.corner.len(), String::new());
            l.extend(r.cells.iter().cloned());
            o.push_str(&l.join("\t"));
            o.push('\n');
        }
        for n in &self.notes {
            o.push_str(n);
            o.push('\n');
        }
        o
    }
    pub fn to_html(&self) -> String {
        let e = html_esc;
        let mut o = format!("<table class=\"pt\"><caption>{}</caption><thead>", e(&self.title));
        let nl = self.corner.len();
        if !self.groups.is_empty() {
            o.push_str(&format!("<tr><th colspan=\"{}\" class=\"c\"></th>", nl));
            for (g, n) in &self.groups {
                o.push_str(&format!("<th colspan=\"{}\">{}</th>", n, e(g)));
            }
            o.push_str("</tr>");
        }
        o.push_str("<tr>");
        for c in &self.corner {
            o.push_str(&format!("<th class=\"c\">{}</th>", e(c)));
        }
        for h in &self.headers {
            o.push_str(&format!("<th>{}</th>", e(h)));
        }
        o.push_str("</tr></thead><tbody>");
        for r in &self.rows {
            o.push_str("<tr>");
            for j in 0..nl {
                o.push_str(&format!("<th class=\"l\">{}</th>", e(r.labels.get(j).map(|s| s.as_str()).unwrap_or(""))));
            }
            for c in &r.cells {
                o.push_str(&format!("<td>{}</td>", e(c)));
            }
            o.push_str("</tr>");
        }
        o.push_str("</tbody></table>");
        for n in &self.notes {
            o.push_str(&format!("<div class=\"note\">{}</div>", e(n)));
        }
        o
    }
}

pub fn html_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[derive(Clone, Debug)]
pub struct BoxStat {
    pub lo: f64,
    pub q1: f64,
    pub med: f64,
    pub q3: f64,
    pub hi: f64,
    pub outliers: Vec<f64>,
    pub n: usize,
}

#[derive(Clone, Debug)]
pub enum ChartKind {
    Bars { labels: Vec<String>, values: Vec<f64> },
    Histogram { edges: Vec<f64>, counts: Vec<f64>, mean: f64, sd: f64, n: usize, normal: bool },
    Scatter { pts: Vec<(f64, f64)>, fit: Option<(f64, f64, f64)> },
    Boxes { groups: Vec<(String, BoxStat)> },
    Pie { labels: Vec<String>, values: Vec<f64> },
}

#[derive(Clone, Debug)]
pub struct Chart {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub kind: ChartKind,
}

#[derive(Clone, Debug)]
pub enum Part {
    Table(Table),
    Chart(Chart),
    Text(String),
    Warn(String),
}
impl Part {
    pub fn title(&self) -> String {
        match self {
            Part::Table(t) => t.title.clone(),
            Part::Chart(c) => {
                if c.title.is_empty() {
                    "Chart".into()
                } else {
                    c.title.clone()
                }
            }
            Part::Text(_) => "Notes".into(),
            Part::Warn(_) => "Warnings".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: u64,
    pub name: String,
    pub syntax: String,
    pub parts: Vec<Part>,
}

// ---------------------------------------------------------------- table drawing

const BORDER: Color32 = Color32::from_rgb(196, 202, 210);
const HEAD_FILL: Color32 = Color32::from_rgb(236, 240, 246);
const LABEL_FILL: Color32 = Color32::from_rgb(246, 248, 251);
const TEXT: Color32 = Color32::from_rgb(28, 32, 38);

fn is_numberish(s: &str) -> bool {
    let t = s.trim().trim_end_matches(|c| c == '*' || c == 'a' || c == 'b' || c == 'c');
    !t.is_empty() && (t.replace(',', "").parse::<f64>().is_ok() || t == "." || t.starts_with('<'))
}

pub fn table_ui(ui: &mut egui::Ui, t: &Table, selected: bool) -> egui::Response {
    let font = FontId::proportional(12.5);
    let tfont = FontId::proportional(14.5);
    let nfont = FontId::proportional(11.5);
    let pad = 14.0;
    let nl = t.corner.len().max(1);
    let painter = ui.painter().clone();
    let w_of = |s: &str| painter.layout_no_wrap(s.to_string(), font.clone(), TEXT).size().x;
    let mut lw: Vec<f32> = (0..nl).map(|j| w_of(t.corner.get(j).map(|s| s.as_str()).unwrap_or("")) + pad).collect();
    for r in &t.rows {
        for j in 0..nl {
            if let Some(s) = r.labels.get(j) {
                lw[j] = lw[j].max(w_of(s) + pad);
            }
        }
    }
    let nc = t.headers.len();
    let mut cw: Vec<f32> = t.headers.iter().map(|h| (w_of(h) + pad).clamp(52.0, 118.0)).collect();
    for r in &t.rows {
        for (i, c) in r.cells.iter().enumerate().take(nc) {
            cw[i] = cw[i].max(w_of(c) + pad);
        }
    }
    let mut gi = 0;
    for (g, n) in &t.groups {
        if *n > 0 && gi + n <= nc {
            let span: f32 = cw[gi..gi + n].iter().sum();
            let need = w_of(g) + pad;
            if need > span {
                let extra = (need - span) / *n as f32;
                for w in &mut cw[gi..gi + n] {
                    *w += extra;
                }
            }
        }
        gi += n;
    }
    let head_galleys: Vec<_> = t.headers.iter().enumerate().map(|(i, h)| painter.layout(h.clone(), font.clone(), TEXT, cw[i] - 8.0)).collect();
    let head_h = head_galleys.iter().map(|g| g.size().y).fold(16.0, f32::max) + 8.0;
    let group_h = if t.groups.is_empty() { 0.0 } else { 22.0 };
    let row_h = 20.0;
    let label_w: f32 = lw.iter().sum();
    let total_w = label_w + cw.iter().sum::<f32>();
    let title_g = painter.layout(t.title.clone(), tfont, TEXT, total_w.max(200.0));
    let title_h = title_g.size().y + 8.0;
    let note_gs: Vec<_> = t.notes.iter().map(|n| painter.layout(n.clone(), nfont.clone(), Color32::from_gray(70), total_w.max(260.0))).collect();
    let notes_h: f32 = note_gs.iter().map(|g| g.size().y + 2.0).sum::<f32>() + if t.notes.is_empty() { 0.0 } else { 4.0 };
    let total_h = title_h + group_h + head_h + row_h * t.rows.len() as f32 + notes_h;
    let width = total_w.max(title_g.size().x);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width + 4.0, total_h + 4.0), Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter_at(rect.expand(2.0));
    if selected {
        p.rect_stroke(rect.expand(1.0), 0.0, Stroke::new(1.5, Color32::from_rgb(70, 120, 200)), StrokeKind::Inside);
    }
    let x0 = rect.min.x + 2.0;
    let mut y = rect.min.y + 2.0;
    let tw = title_g.size().x;
    p.galley(Pos2::new(x0 + (total_w - tw).max(0.0) / 2.0, y + 2.0), title_g, TEXT);
    y += title_h;
    let top = y;
    let hy = top + group_h;
    let body_top = hy + head_h;
    let bottom = body_top + row_h * t.rows.len() as f32;
    p.rect_filled(Rect::from_min_max(Pos2::new(x0, top), Pos2::new(x0 + total_w, body_top)), 0.0, HEAD_FILL);
    p.rect_filled(Rect::from_min_max(Pos2::new(x0, body_top), Pos2::new(x0 + label_w, bottom)), 0.0, LABEL_FILL);
    p.rect_filled(Rect::from_min_max(Pos2::new(x0 + label_w, body_top), Pos2::new(x0 + total_w, bottom)), 0.0, Color32::WHITE);
    let line = Stroke::new(1.0, BORDER);
    let dark = Stroke::new(1.0, Color32::from_gray(150));
    if !t.groups.is_empty() {
        let mut x = x0 + label_w;
        let mut ci = 0;
        for (g, n) in &t.groups {
            let w: f32 = if ci + n <= nc { cw[ci..ci + n].iter().sum() } else { 0.0 };
            if !g.is_empty() {
                p.text(Pos2::new(x + w / 2.0, top + group_h / 2.0), Align2::CENTER_CENTER, g, font.clone(), TEXT);
                p.line_segment([Pos2::new(x + 3.0, top + group_h), Pos2::new(x + w - 3.0, top + group_h)], line);
            }
            x += w;
            ci += n;
        }
    }
    let mut x = x0;
    for j in 0..nl {
        p.text(Pos2::new(x + 6.0, body_top - 6.0), Align2::LEFT_BOTTOM, t.corner.get(j).map(|s| s.as_str()).unwrap_or(""), font.clone(), TEXT);
        x += lw[j];
    }
    for (i, g) in head_galleys.into_iter().enumerate() {
        let (gw, gh) = (g.size().x, g.size().y);
        p.galley(Pos2::new(x + (cw[i] - gw) / 2.0, body_top - gh - 4.0), g, TEXT);
        x += cw[i];
    }
    p.line_segment([Pos2::new(x0, body_top), Pos2::new(x0 + total_w, body_top)], dark);
    for (ri, r) in t.rows.iter().enumerate() {
        let ry = body_top + row_h * ri as f32;
        let mut x = x0;
        let mut drew = false;
        for j in 0..nl {
            let s = r.labels.get(j).map(|s| s.as_str()).unwrap_or("");
            if ri > 0 && !drew && (!s.is_empty() || j == nl - 1) {
                p.line_segment([Pos2::new(x, ry), Pos2::new(x0 + total_w, ry)], line);
                drew = true;
            }
            if !s.is_empty() {
                p.text(Pos2::new(x + 6.0, ry + row_h / 2.0), Align2::LEFT_CENTER, s, font.clone(), TEXT);
            }
            x += lw[j];
        }
        for (i, c) in r.cells.iter().enumerate().take(nc) {
            if is_numberish(c) {
                p.text(Pos2::new(x + cw[i] - 7.0, ry + row_h / 2.0), Align2::RIGHT_CENTER, c, font.clone(), TEXT);
            } else {
                p.text(Pos2::new(x + 6.0, ry + row_h / 2.0), Align2::LEFT_CENTER, c, font.clone(), TEXT);
            }
            x += cw[i];
        }
    }
    let mut x = x0;
    p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], line);
    for w in lw.iter() {
        x += w;
    }
    p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], dark);
    for w in cw.iter() {
        x += w;
        p.line_segment([Pos2::new(x, hy), Pos2::new(x, bottom)], line);
    }
    p.line_segment([Pos2::new(x0 + total_w, top), Pos2::new(x0 + total_w, bottom)], line);
    p.line_segment([Pos2::new(x0, top), Pos2::new(x0 + total_w, top)], line);
    p.line_segment([Pos2::new(x0, bottom), Pos2::new(x0 + total_w, bottom)], line);
    let mut ny = bottom + 4.0;
    for g in note_gs {
        let h = g.size().y;
        p.galley(Pos2::new(x0, ny), g, Color32::from_gray(70));
        ny += h + 2.0;
    }
    resp
}

// ---------------------------------------------------------------- charts as primitives

pub const CW: f32 = 620.0;
pub const CH: f32 = 430.0;

#[derive(Clone, Debug)]
pub enum Prim {
    Rect { r: [f32; 4], fill: Color32, stroke: Option<Color32> },
    Line { a: [f32; 2], b: [f32; 2], color: Color32, w: f32 },
    Circle { c: [f32; 2], r: f32, fill: Option<Color32>, stroke: Option<Color32> },
    Fan { pts: Vec<[f32; 2]>, fill: Color32 },
    Polyline { pts: Vec<[f32; 2]>, color: Color32, w: f32 },
    Text { p: [f32; 2], s: String, size: f32, anchor: Align2, color: Color32, vertical: bool },
}

pub const PALETTE: [Color32; 10] = [
    Color32::from_rgb(64, 112, 176),
    Color32::from_rgb(230, 140, 56),
    Color32::from_rgb(86, 164, 98),
    Color32::from_rgb(198, 72, 72),
    Color32::from_rgb(142, 108, 180),
    Color32::from_rgb(150, 112, 82),
    Color32::from_rgb(214, 122, 178),
    Color32::from_rgb(128, 128, 128),
    Color32::from_rgb(186, 186, 62),
    Color32::from_rgb(64, 178, 190),
];
const BAR: Color32 = Color32::from_rgb(86, 128, 186);
const BAR_EDGE: Color32 = Color32::from_rgb(40, 70, 120);
const AXIS: Color32 = Color32::from_rgb(70, 70, 70);
const GRID: Color32 = Color32::from_rgb(228, 230, 234);

fn nice_num(x: f64, round: bool) -> f64 {
    if x <= 0.0 || !x.is_finite() {
        return 1.0;
    }
    let e = x.log10().floor();
    let f = x / 10f64.powf(e);
    let nf = if round {
        if f < 1.5 {
            1.0
        } else if f < 3.0 {
            2.0
        } else if f < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    nf * 10f64.powf(e)
}

pub fn nice_ticks(lo: f64, hi: f64, n: usize) -> (f64, f64, Vec<f64>) {
    let (lo, hi) = if hi > lo { (lo, hi) } else { (lo - 0.5, lo + 0.5) };
    let range = nice_num(hi - lo, false);
    let step = nice_num(range / (n.max(2) - 1) as f64, true);
    let nlo = (lo / step).floor() * step;
    let nhi = (hi / step).ceil() * step;
    let mut t = vec![];
    let mut k = 0;
    loop {
        let v = nlo + step * k as f64;
        if v > nhi + step * 0.5 || k > 60 {
            break;
        }
        t.push(if v.abs() < step * 1e-9 { 0.0 } else { v });
        k += 1;
    }
    (nlo, nhi, t)
}

fn tick_fmt(v: f64, ticks: &[f64]) -> String {
    let step = if ticks.len() > 1 { (ticks[1] - ticks[0]).abs() } else { 1.0 };
    let d = if step >= 1.0 { 0 } else { (-step.log10().floor()) as usize };
    let s = format!("{:.*}", d.min(6), v);
    if v.abs() >= 10000.0 && d == 0 {
        let neg = v < 0.0;
        let digits: String = s.trim_start_matches('-').to_string();
        let mut out = String::new();
        for (i, ch) in digits.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        let r: String = out.chars().rev().collect();
        if neg {
            format!("-{}", r)
        } else {
            r
        }
    } else {
        s
    }
}

fn txt(p: [f32; 2], s: impl Into<String>, size: f32, anchor: Align2) -> Prim {
    Prim::Text { p, s: s.into(), size, anchor, color: Color32::from_gray(30), vertical: false }
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", t)
    }
}

struct Axes {
    l: f32,
    t: f32,
    r: f32,
    b: f32,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}
impl Axes {
    fn px(&self, x: f64) -> f32 {
        self.l + ((x - self.x0) / (self.x1 - self.x0)) as f32 * (self.r - self.l)
    }
    fn py(&self, y: f64) -> f32 {
        self.b - ((y - self.y0) / (self.y1 - self.y0)) as f32 * (self.b - self.t)
    }
}

fn y_axis(o: &mut Vec<Prim>, a: &Axes, ticks: &[f64], label: &str) {
    for &tv in ticks {
        let y = a.py(tv);
        o.push(Prim::Line { a: [a.l, y], b: [a.r, y], color: GRID, w: 1.0 });
        o.push(Prim::Line { a: [a.l - 4.0, y], b: [a.l, y], color: AXIS, w: 1.0 });
        o.push(txt([a.l - 7.0, y], tick_fmt(tv, ticks), 11.0, Align2::RIGHT_CENTER));
    }
    o.push(Prim::Line { a: [a.l, a.t], b: [a.l, a.b], color: AXIS, w: 1.0 });
    o.push(Prim::Text { p: [16.0, (a.t + a.b) / 2.0], s: label.to_string(), size: 12.0, anchor: Align2::CENTER_CENTER, color: Color32::from_gray(30), vertical: true });
}

fn x_axis_numeric(o: &mut Vec<Prim>, a: &Axes, ticks: &[f64], label: &str) {
    for &tv in ticks {
        let x = a.px(tv);
        o.push(Prim::Line { a: [x, a.b], b: [x, a.b + 4.0], color: AXIS, w: 1.0 });
        o.push(txt([x, a.b + 7.0], tick_fmt(tv, ticks), 11.0, Align2::CENTER_TOP));
    }
    o.push(Prim::Line { a: [a.l, a.b], b: [a.r, a.b], color: AXIS, w: 1.0 });
    o.push(txt([(a.l + a.r) / 2.0, CH - 14.0], label, 12.0, Align2::CENTER_CENTER));
}

pub fn chart_prims(ch: &Chart) -> Vec<Prim> {
    let mut o = vec![Prim::Rect { r: [0.0, 0.0, CW, CH], fill: Color32::WHITE, stroke: Some(Color32::from_gray(190)) }];
    o.push(txt([CW / 2.0, 18.0], ch.title.clone(), 14.0, Align2::CENTER_CENTER));
    match &ch.kind {
        ChartKind::Bars { labels, values } => {
            let maxv = values.iter().copied().filter(|v| v.is_finite()).fold(0.0f64, f64::max);
            let minv = values.iter().copied().filter(|v| v.is_finite()).fold(0.0f64, f64::min);
            let (y0, y1, ticks) = nice_ticks(minv, if maxv > 0.0 { maxv * 1.05 } else { 1.0 }, 6);
            let a = Axes { l: 78.0, t: 42.0, r: CW - 24.0, b: CH - 62.0, x0: 0.0, x1: 1.0, y0, y1 };
            y_axis(&mut o, &a, &ticks, &ch.y_label);
            let n = labels.len().max(1);
            let slot = (a.r - a.l) / n as f32;
            let maxc = ((slot / 6.5) as usize).max(3);
            for (i, (l, v)) in labels.iter().zip(values).enumerate() {
                let cx = a.l + slot * (i as f32 + 0.5);
                let bw = (slot * 0.62).min(90.0);
                if v.is_finite() {
                    let (ya, yb) = (a.py(0.0), a.py(*v));
                    o.push(Prim::Rect { r: [cx - bw / 2.0, ya.min(yb), bw, (ya - yb).abs()], fill: BAR, stroke: Some(BAR_EDGE) });
                }
                o.push(txt([cx, a.b + 7.0], trunc(l, maxc), 11.0, Align2::CENTER_TOP));
            }
            o.push(Prim::Line { a: [a.l, a.py(0.0)], b: [a.r, a.py(0.0)], color: AXIS, w: 1.0 });
            o.push(txt([(a.l + a.r) / 2.0, CH - 14.0], ch.x_label.clone(), 12.0, Align2::CENTER_CENTER));
        }
        ChartKind::Histogram { edges, counts, mean, sd, n, normal } => {
            let maxc = counts.iter().copied().fold(0.0f64, f64::max);
            let bw = if edges.len() > 1 { edges[1] - edges[0] } else { 1.0 };
            let peak = if *normal && *sd > 0.0 { *n as f64 * bw / (sd * (2.0 * std::f64::consts::PI).sqrt()) } else { 0.0 };
            let (y0, y1, yt) = nice_ticks(0.0, maxc.max(peak) * 1.05 + 0.5, 6);
            let first = edges.first().copied().unwrap_or(0.0);
            let last = edges.last().copied().unwrap_or(1.0);
            let (x0, x1, xt) = nice_ticks(first, last, 7);
            let a = Axes { l: 70.0, t: 42.0, r: CW - 150.0, b: CH - 58.0, x0, x1, y0, y1 };
            y_axis(&mut o, &a, &yt, "Frequency");
            for (i, c) in counts.iter().enumerate() {
                if i + 1 >= edges.len() {
                    break;
                }
                let xa = a.px(edges[i]);
                let xb = a.px(edges[i + 1]);
                let ya = a.py(*c);
                if *c > 0.0 {
                    o.push(Prim::Rect { r: [xa, ya, xb - xa, a.b - ya], fill: BAR, stroke: Some(BAR_EDGE) });
                }
            }
            x_axis_numeric(&mut o, &a, &xt, &ch.x_label);
            if *normal && *sd > 0.0 {
                let mut pts = vec![];
                for k in 0..=120 {
                    let x = x0 + (x1 - x0) * k as f64 / 120.0;
                    let z = (x - mean) / sd;
                    let dens = (-0.5 * z * z).exp() / (sd * (2.0 * std::f64::consts::PI).sqrt());
                    pts.push([a.px(x), a.py(dens * *n as f64 * bw)]);
                }
                o.push(Prim::Polyline { pts, color: Color32::from_rgb(20, 20, 20), w: 1.6 });
            }
            let lx = CW - 138.0;
            o.push(txt([lx, 60.0], format!("Mean = {:.2}", mean), 11.5, Align2::LEFT_CENTER));
            o.push(txt([lx, 78.0], format!("Std. Dev. = {:.3}", sd), 11.5, Align2::LEFT_CENTER));
            o.push(txt([lx, 96.0], format!("N = {}", n), 11.5, Align2::LEFT_CENTER));
        }
        ChartKind::Scatter { pts, fit } => {
            let (mut xa, mut xb, mut ya, mut yb) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for (x, y) in pts {
                xa = xa.min(*x);
                xb = xb.max(*x);
                ya = ya.min(*y);
                yb = yb.max(*y);
            }
            if pts.is_empty() {
                xa = 0.0;
                xb = 1.0;
                ya = 0.0;
                yb = 1.0;
            }
            let (x0, x1, xt) = nice_ticks(xa, xb, 7);
            let (y0, y1, yt) = nice_ticks(ya, yb, 6);
            let a = Axes { l: 78.0, t: 42.0, r: CW - 28.0, b: CH - 58.0, x0, x1, y0, y1 };
            y_axis(&mut o, &a, &yt, &ch.y_label);
            x_axis_numeric(&mut o, &a, &xt, &ch.x_label);
            for (x, y) in pts {
                o.push(Prim::Circle { c: [a.px(*x), a.py(*y)], r: 3.2, fill: Some(Color32::from_rgba_unmultiplied(86, 128, 186, 90)), stroke: Some(Color32::from_rgb(50, 90, 150)) });
            }
            if let Some((b0, b1, r2)) = fit {
                let p1 = [a.px(x0), a.py(b0 + b1 * x0)];
                let p2 = [a.px(x1), a.py(b0 + b1 * x1)];
                o.push(Prim::Line { a: p1, b: p2, color: Color32::from_rgb(200, 60, 40), w: 1.6 });
                o.push(txt([a.r - 6.0, a.t + 10.0], format!("R² Linear = {:.3}", r2), 11.5, Align2::RIGHT_CENTER));
            }
        }
        ChartKind::Boxes { groups } => {
            let mut lo = f64::MAX;
            let mut hi = f64::MIN;
            for (_, b) in groups {
                lo = lo.min(b.lo);
                hi = hi.max(b.hi);
                for x in &b.outliers {
                    lo = lo.min(*x);
                    hi = hi.max(*x);
                }
            }
            if groups.is_empty() {
                lo = 0.0;
                hi = 1.0;
            }
            let (y0, y1, yt) = nice_ticks(lo, hi, 6);
            let a = Axes { l: 78.0, t: 42.0, r: CW - 24.0, b: CH - 62.0, x0: 0.0, x1: 1.0, y0, y1 };
            y_axis(&mut o, &a, &yt, &ch.y_label);
            o.push(Prim::Line { a: [a.l, a.b], b: [a.r, a.b], color: AXIS, w: 1.0 });
            let n = groups.len().max(1);
            let slot = (a.r - a.l) / n as f32;
            for (i, (name, b)) in groups.iter().enumerate() {
                let cx = a.l + slot * (i as f32 + 0.5);
                let w = (slot * 0.4).min(70.0);
                let (yq1, yq3, ym) = (a.py(b.q1), a.py(b.q3), a.py(b.med));
                o.push(Prim::Line { a: [cx, a.py(b.lo)], b: [cx, yq1], color: AXIS, w: 1.0 });
                o.push(Prim::Line { a: [cx, a.py(b.hi)], b: [cx, yq3], color: AXIS, w: 1.0 });
                o.push(Prim::Line { a: [cx - w / 4.0, a.py(b.lo)], b: [cx + w / 4.0, a.py(b.lo)], color: AXIS, w: 1.0 });
                o.push(Prim::Line { a: [cx - w / 4.0, a.py(b.hi)], b: [cx + w / 4.0, a.py(b.hi)], color: AXIS, w: 1.0 });
                o.push(Prim::Rect { r: [cx - w / 2.0, yq3, w, (yq1 - yq3).max(1.0)], fill: Color32::from_rgb(170, 196, 230), stroke: Some(BAR_EDGE) });
                o.push(Prim::Line { a: [cx - w / 2.0, ym], b: [cx + w / 2.0, ym], color: Color32::from_rgb(20, 20, 20), w: 2.2 });
                for x in &b.outliers {
                    o.push(Prim::Circle { c: [cx, a.py(*x)], r: 3.0, fill: None, stroke: Some(Color32::from_rgb(60, 60, 60)) });
                }
                o.push(txt([cx, a.b + 7.0], trunc(name, ((slot / 6.5) as usize).max(3)), 11.0, Align2::CENTER_TOP));
                o.push(txt([cx, a.b + 22.0], format!("N = {}", b.n), 10.0, Align2::CENTER_TOP));
            }
            o.push(txt([(a.l + a.r) / 2.0, CH - 14.0], ch.x_label.clone(), 12.0, Align2::CENTER_CENTER));
        }
        ChartKind::Pie { labels, values } => {
            let total: f64 = values.iter().sum();
            let c = [CW / 2.0 - 90.0, CH / 2.0 + 12.0];
            let r = 150.0f32;
            let mut ang = -std::f64::consts::FRAC_PI_2;
            for (i, v) in values.iter().enumerate() {
                if total <= 0.0 {
                    break;
                }
                let sweep = v / total * std::f64::consts::TAU;
                let steps = ((sweep / 0.05).ceil() as usize).max(2);
                let mut pts = vec![c];
                for k in 0..=steps {
                    let t = ang + sweep * k as f64 / steps as f64;
                    pts.push([c[0] + r * t.cos() as f32, c[1] + r * t.sin() as f32]);
                }
                o.push(Prim::Fan { pts, fill: PALETTE[i % PALETTE.len()] });
                let mid = ang + sweep / 2.0;
                if v / total > 0.04 {
                    o.push(Prim::Text {
                        p: [c[0] + r * 0.65 * mid.cos() as f32, c[1] + r * 0.65 * mid.sin() as f32],
                        s: format!("{:.1}%", v / total * 100.0),
                        size: 11.0,
                        anchor: Align2::CENTER_CENTER,
                        color: Color32::WHITE,
                        vertical: false,
                    });
                }
                ang += sweep;
            }
            let lx = CW - 190.0;
            o.push(txt([lx, 56.0], ch.x_label.clone(), 12.0, Align2::LEFT_CENTER));
            for (i, l) in labels.iter().enumerate().take(18) {
                let y = 78.0 + i as f32 * 19.0;
                o.push(Prim::Rect { r: [lx, y - 6.0, 12.0, 12.0], fill: PALETTE[i % PALETTE.len()], stroke: None });
                o.push(txt([lx + 18.0, y], trunc(l, 24), 11.0, Align2::LEFT_CENTER));
            }
        }
    }
    o
}

pub fn paint_prims(painter: &egui::Painter, origin: Pos2, prims: &[Prim]) {
    let p = |q: [f32; 2]| Pos2::new(origin.x + q[0], origin.y + q[1]);
    for pr in prims {
        match pr {
            Prim::Rect { r, fill, stroke } => {
                let rect = Rect::from_min_size(p([r[0], r[1]]), Vec2::new(r[2], r[3]));
                painter.rect_filled(rect, 0.0, *fill);
                if let Some(s) = stroke {
                    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, *s), StrokeKind::Inside);
                }
            }
            Prim::Line { a, b, color, w } => {
                painter.line_segment([p(*a), p(*b)], Stroke::new(*w, *color));
            }
            Prim::Circle { c, r, fill, stroke } => {
                if let Some(f) = fill {
                    painter.circle_filled(p(*c), *r, *f);
                }
                if let Some(s) = stroke {
                    painter.circle_stroke(p(*c), *r, Stroke::new(1.0, *s));
                }
            }
            Prim::Fan { pts, fill } => {
                if pts.len() >= 3 {
                    let c = p(pts[0]);
                    for w in pts[1..].windows(2) {
                        painter.add(egui::Shape::convex_polygon(vec![c, p(w[0]), p(w[1])], *fill, Stroke::new(0.6, *fill)));
                    }
                }
            }
            Prim::Polyline { pts, color, w } => {
                painter.add(egui::Shape::line(pts.iter().map(|q| p(*q)).collect(), Stroke::new(*w, *color)));
            }
            Prim::Text { p: q, s, size, anchor, color, vertical } => {
                if *vertical {
                    let g = painter.layout_no_wrap(s.clone(), FontId::proportional(*size), *color);
                    let (w, h) = (g.size().x, g.size().y);
                    let pos = Pos2::new(origin.x + q[0] - h / 2.0, origin.y + q[1] + w / 2.0);
                    painter.add(egui::epaint::TextShape::new(pos, g, *color).with_angle(-std::f32::consts::FRAC_PI_2));
                } else {
                    painter.text(p(*q), *anchor, s, FontId::proportional(*size), *color);
                }
            }
        }
    }
}

pub fn chart_ui(ui: &mut egui::Ui, ch: &Chart, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(CW, CH), Sense::click());
    if ui.is_rect_visible(rect) {
        let prims = chart_prims(ch);
        paint_prims(&ui.painter_at(rect.expand(1.0)), rect.min, &prims);
        if selected {
            ui.painter().rect_stroke(rect.expand(2.0), 0.0, Stroke::new(1.5, Color32::from_rgb(70, 120, 200)), StrokeKind::Outside);
        }
    }
    resp
}

fn hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

pub fn chart_svg(ch: &Chart) -> String {
    let mut o = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" font-family=\"Segoe UI, Arial, sans-serif\">", CW, CH);
    for pr in chart_prims(ch) {
        match pr {
            Prim::Rect { r, fill, stroke } => o.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" stroke=\"{}\"/>",
                r[0],
                r[1],
                r[2],
                r[3],
                hex(fill),
                stroke.map(hex).unwrap_or_else(|| "none".into())
            )),
            Prim::Line { a, b, color, w } => o.push_str(&format!("<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"{}\"/>", a[0], a[1], b[0], b[1], hex(color), w)),
            Prim::Circle { c, r, fill, stroke } => o.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{}\" fill=\"{}\" fill-opacity=\"0.4\" stroke=\"{}\"/>",
                c[0],
                c[1],
                r,
                fill.map(hex).unwrap_or_else(|| "none".into()),
                stroke.map(hex).unwrap_or_else(|| "none".into())
            )),
            Prim::Fan { pts, fill } => {
                let s: Vec<String> = pts.iter().map(|q| format!("{:.1},{:.1}", q[0], q[1])).collect();
                o.push_str(&format!("<polygon points=\"{}\" fill=\"{}\"/>", s.join(" "), hex(fill)));
            }
            Prim::Polyline { pts, color, w } => {
                let s: Vec<String> = pts.iter().map(|q| format!("{:.1},{:.1}", q[0], q[1])).collect();
                o.push_str(&format!("<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>", s.join(" "), hex(color), w));
            }
            Prim::Text { p, s, size, anchor, color, vertical } => {
                let ta = match anchor.x() {
                    egui::Align::Min => "start",
                    egui::Align::Center => "middle",
                    egui::Align::Max => "end",
                };
                let bl = match anchor.y() {
                    egui::Align::Min => "hanging",
                    egui::Align::Center => "middle",
                    egui::Align::Max => "auto",
                };
                let tr = if vertical { format!(" transform=\"rotate(-90 {:.1} {:.1})\"", p[0], p[1]) } else { String::new() };
                o.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{}\" text-anchor=\"{}\" dominant-baseline=\"{}\" fill=\"{}\"{}>{}</text>",
                    p[0],
                    p[1],
                    size,
                    ta,
                    bl,
                    hex(color),
                    tr,
                    html_esc(&s)
                ));
            }
        }
    }
    o.push_str("</svg>");
    o
}

pub fn blocks_html(blocks: &[Block]) -> String {
    let mut o = String::from(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>StatDesk Output</title><style>\
body{font-family:'Segoe UI',Arial,sans-serif;margin:28px;color:#1c2026}\
h2{font-size:18px;margin:28px 0 6px;border-bottom:1px solid #d8dde4;padding-bottom:4px}\
pre{background:#f6f8fb;border:1px solid #e1e5ea;padding:8px;font-size:12px}\
table.pt{border-collapse:collapse;margin:14px 0 4px;font-size:13px}\
table.pt caption{font-weight:600;font-size:14px;padding:6px}\
table.pt th,table.pt td{border:1px solid #c4cad2;padding:3px 8px}\
table.pt thead th{background:#ecf0f6;font-weight:normal}\
table.pt th.l{background:#f6f8fb;text-align:left;font-weight:normal}\
table.pt td{text-align:right}\
.note{font-size:12px;color:#555}.warn{background:#fff6e0;border:1px solid #e8cf88;padding:8px;font-size:13px}\
</style></head><body>",
    );
    for b in blocks {
        o.push_str(&format!("<h2>{}</h2>", html_esc(&b.name)));
        if !b.syntax.is_empty() {
            o.push_str(&format!("<pre>{}</pre>", html_esc(&b.syntax)));
        }
        for p in &b.parts {
            match p {
                Part::Table(t) => o.push_str(&t.to_html()),
                Part::Chart(c) => {
                    o.push_str("<div>");
                    o.push_str(&chart_svg(c));
                    o.push_str("</div>");
                }
                Part::Text(t) => o.push_str(&format!("<p>{}</p>", html_esc(t))),
                Part::Warn(t) => o.push_str(&format!("<div class=\"warn\">{}</div>", html_esc(t))),
            }
        }
    }
    o.push_str("</body></html>");
    o
}

pub fn blocks_text(blocks: &[Block]) -> String {
    let mut o = String::new();
    for b in blocks {
        o.push_str(&format!("==== {} ====\n", b.name));
        if !b.syntax.is_empty() {
            o.push_str(&b.syntax);
            o.push_str("\n\n");
        }
        for p in &b.parts {
            match p {
                Part::Table(t) => o.push_str(&t.to_tsv()),
                Part::Chart(c) => o.push_str(&format!("[Chart: {}]\n", c.title)),
                Part::Text(t) => o.push_str(&format!("{}\n", t)),
                Part::Warn(t) => o.push_str(&format!("Warning: {}\n", t)),
            }
            o.push('\n');
        }
    }
    o
}
