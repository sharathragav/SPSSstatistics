#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! StatDesk: a lightweight statistics workbench (data editor, analyses, output viewer, syntax).

mod data;
mod dialogs;
mod expr;
mod output;
mod stats;
mod syntax;
mod views;

use data::Dataset;
use dialogs::{Action, ComputeDlg, Dialog, Kind, SelectDlg};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use output::{Block, Part};
use std::path::PathBuf;
use syntax::Command;

pub const APP_NAME: &str = "StatDesk";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Data,
    Variables,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelMode {
    Cell,
    Row,
    Col,
}

#[derive(Clone)]
pub enum Pending {
    New,
    Open,
    OpenPath(PathBuf),
    Sample,
    Exit,
}

pub struct Edit {
    pub r: usize,
    pub c: usize,
    pub text: String,
    pub focus: bool,
}

pub struct ValueLabelsDlg {
    pub var: usize,
    pub labels: Vec<(String, String)>,
    pub value: String,
    pub label: String,
    pub sel: Option<usize>,
}

pub struct MissingDlg {
    pub var: usize,
    pub on: bool,
    pub vals: [String; 3],
}

pub struct GotoDlg {
    pub case: bool,
    pub text: String,
    pub var: usize,
}

pub struct FindDlg {
    pub text: String,
    pub whole: bool,
    pub msg: String,
}

pub struct App {
    pub ds: Dataset,
    pub path: Option<PathBuf>,
    pub ds_title: String,
    pub dirty: bool,
    pub undo: Vec<Dataset>,
    pub redo: Vec<Dataset>,
    pub view: View,
    pub sel: (usize, usize),
    pub sel_mode: SelMode,
    pub editing: Option<Edit>,
    pub top_row: usize,
    pub left_col: usize,
    pub scroll_acc: Vec2,
    pub labels_on: bool,
    pub grid_lines: bool,
    pub bar_text: String,
    pub bar_for: (usize, usize, usize),
    pub new_var_name: String,
    pub out: Vec<Block>,
    pub out_open: bool,
    pub out_sel: Option<(usize, Option<usize>)>,
    pub out_scroll: Option<(usize, Option<usize>)>,
    pub out_focus: bool,
    pub next_id: u64,
    pub syn_text: String,
    pub syn_open: bool,
    pub syn_focus: bool,
    pub syn_path: Option<PathBuf>,
    pub syn_cursor: (usize, usize),
    pub dialog: Option<Dialog>,
    pub last_dialog: Option<Dialog>,
    pub compute: Option<ComputeDlg>,
    pub select: Option<SelectDlg>,
    pub vlabels: Option<ValueLabelsDlg>,
    pub missing_dlg: Option<MissingDlg>,
    pub goto: Option<GotoDlg>,
    pub find: Option<FindDlg>,
    pub welcome: bool,
    pub no_welcome: bool,
    pub about: bool,
    pub help: bool,
    pub message: Option<(String, String)>,
    pub pending: Option<Pending>,
    pub allow_close: bool,
    pub status: String,
    pub recent: Vec<PathBuf>,
    pub shot: Option<PathBuf>,
    pub frames: u32,
    pub shot_sent: bool,
    pub last_title: String,
    pub untitled_n: usize,
    pub shot_output: bool,
}

fn config_dir() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("XDG_CONFIG_HOME")).map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("StatDesk"))
}

fn setup_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates: [(&str, &str, bool); 4] = [
        ("ui", "C:\\Windows\\Fonts\\segoeui.ttf", false),
        ("mono", "C:\\Windows\\Fonts\\consola.ttf", true),
        ("ui", "/System/Library/Fonts/SFNS.ttf", false),
        ("ui", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", false),
    ];
    let mut have_ui = false;
    for (name, path, mono) in candidates {
        if (mono || !have_ui) && std::path::Path::new(path).exists() {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.font_data.insert(name.to_string(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
                let fam = if mono { egui::FontFamily::Monospace } else { egui::FontFamily::Proportional };
                fonts.families.entry(fam).or_default().insert(0, name.to_string());
                if !mono {
                    have_ui = true;
                }
            }
        }
    }
    ctx.set_fonts(fonts);
    let mut v = egui::Visuals::light();
    v.panel_fill = Color32::from_rgb(244, 245, 247);
    v.window_fill = Color32::from_rgb(250, 250, 251);
    v.extreme_bg_color = Color32::WHITE;
    v.faint_bg_color = Color32::from_rgb(246, 248, 251);
    v.selection.bg_fill = Color32::from_rgb(196, 220, 250);
    v.selection.stroke = Stroke::new(1.0, Color32::from_rgb(30, 80, 160));
    v.hyperlink_color = Color32::from_rgb(30, 90, 180);
    v.window_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(50) };
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 2], blur: 8, spread: 0, color: Color32::from_black_alpha(40) };
    v.window_corner_radius = egui::CornerRadius::same(4);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(236, 238, 241);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_gray(200));
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(224, 234, 248);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(206, 222, 244);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(6.0, 4.0);
        s.spacing.scroll = egui::style::ScrollStyle::solid();
        s.spacing.button_padding = Vec2::new(8.0, 3.0);
        s.spacing.menu_margin = egui::Margin::symmetric(6, 4);
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        s.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
        s.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
        s.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(17.0));
    });
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>, open: Option<PathBuf>) -> Self {
        setup_style(&cc.egui_ctx);
        let mut recent = vec![];
        let mut no_welcome = false;
        if shot.is_none() {
            if let Some(dir) = config_dir() {
                if let Ok(t) = std::fs::read_to_string(dir.join("recent.txt")) {
                    recent = t.lines().filter(|l| !l.trim().is_empty()).map(PathBuf::from).collect();
                }
                no_welcome = dir.join("no-welcome").exists();
            }
        }
        let mut app = App {
            ds: Dataset::default(),
            path: None,
            ds_title: "Untitled1".into(),
            dirty: false,
            undo: vec![],
            redo: vec![],
            view: View::Data,
            sel: (0, 0),
            sel_mode: SelMode::Cell,
            editing: None,
            top_row: 0,
            left_col: 0,
            scroll_acc: Vec2::ZERO,
            labels_on: false,
            grid_lines: true,
            bar_text: String::new(),
            bar_for: (usize::MAX, usize::MAX, usize::MAX),
            new_var_name: String::new(),
            out: vec![],
            out_open: false,
            out_sel: None,
            out_scroll: None,
            out_focus: false,
            next_id: 1,
            syn_text: String::new(),
            syn_open: false,
            syn_focus: false,
            syn_path: None,
            syn_cursor: (0, 0),
            dialog: None,
            last_dialog: None,
            compute: None,
            select: None,
            vlabels: None,
            missing_dlg: None,
            goto: None,
            find: None,
            welcome: !no_welcome,
            no_welcome,
            about: false,
            help: false,
            message: None,
            pending: None,
            allow_close: false,
            status: String::new(),
            recent,
            shot,
            frames: 0,
            shot_sent: false,
            last_title: String::new(),
            untitled_n: 1,
            shot_output: false,
        };
        if let Some(p) = open {
            app.welcome = false;
            app.open_path(p);
        }
        app
    }

    // ------------------------------------------------------------ data helpers

    pub fn push_undo(&mut self) {
        self.undo.push(self.ds.clone());
        if self.undo.len() > 40 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }
    pub fn do_undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.ds, prev));
            self.status = "Undone.".into();
            self.bar_for = (usize::MAX, usize::MAX, usize::MAX);
        }
    }
    pub fn do_redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.ds, next));
            self.status = "Redone.".into();
            self.bar_for = (usize::MAX, usize::MAX, usize::MAX);
        }
    }

    pub fn set_dataset(&mut self, ds: Dataset, title: String, path: Option<PathBuf>) {
        self.ds = ds;
        self.ds_title = title;
        self.path = path;
        self.dirty = false;
        self.undo.clear();
        self.redo.clear();
        self.sel = (0, 0);
        self.sel_mode = SelMode::Cell;
        self.editing = None;
        self.top_row = 0;
        self.left_col = 0;
        self.bar_for = (usize::MAX, usize::MAX, usize::MAX);
    }

    fn remember_recent(&mut self, p: &PathBuf) {
        self.recent.retain(|x| x != p);
        self.recent.insert(0, p.clone());
        self.recent.truncate(8);
        if self.shot.is_none() {
            if let Some(dir) = config_dir() {
                let _ = std::fs::create_dir_all(&dir);
                let t: Vec<String> = self.recent.iter().map(|p| p.display().to_string()).collect();
                let _ = std::fs::write(dir.join("recent.txt"), t.join("\n"));
            }
        }
    }

    pub fn guarded(&mut self, p: Pending) {
        if self.dirty {
            self.pending = Some(p);
        } else {
            self.do_pending(p);
        }
    }

    fn do_pending(&mut self, p: Pending) {
        match p {
            Pending::New => {
                self.untitled_n += 1;
                let t = format!("Untitled{}", self.untitled_n);
                self.set_dataset(Dataset::default(), t, None);
                self.status = "New dataset.".into();
            }
            Pending::Open => {
                if let Some(p) = rfd::FileDialog::new()
                    .set_title("Open Data")
                    .add_filter("All data files", &["sdv", "csv", "tsv", "txt", "dat", "tab"])
                    .add_filter("StatDesk data (*.sdv)", &["sdv"])
                    .add_filter("Comma separated (*.csv)", &["csv"])
                    .add_filter("Tab delimited (*.tsv, *.txt, *.dat)", &["tsv", "txt", "dat", "tab"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.open_path(p);
                }
            }
            Pending::OpenPath(p) => self.open_path(p),
            Pending::Sample => {
                self.set_dataset(data::sample_dataset(), "workplace_survey".into(), None);
                self.dirty = false;
                self.status = "Opened the sample dataset (150 made-up survey respondents).".into();
            }
            Pending::Exit => {
                self.allow_close = true;
            }
        }
    }

    pub fn open_path(&mut self, p: PathBuf) {
        match data::load_file(&p) {
            Ok(ds) => {
                let title = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Data".into());
                let native = p.extension().map(|e| e.eq_ignore_ascii_case(data::NATIVE_EXT)).unwrap_or(false);
                let n = ds.ncases;
                let k = ds.vars.len();
                self.set_dataset(ds, title, if native { Some(p.clone()) } else { None });
                if !native {
                    self.path = Some(p.clone());
                }
                self.remember_recent(&p);
                self.status = format!("Opened {} ({} cases, {} variables).", p.display(), n, k);
                self.welcome = false;
            }
            Err(e) => self.message = Some(("Couldn't open the file".into(), format!("{}\n\n{}", p.display(), e))),
        }
    }

    pub fn save(&mut self) {
        match self.path.clone() {
            Some(p) => self.save_to(p),
            None => self.save_as(),
        }
    }

    pub fn save_as(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .set_title("Save Data As")
            .set_file_name(format!("{}.{}", self.ds_title, data::NATIVE_EXT))
            .add_filter("StatDesk data (*.sdv)", &["sdv"])
            .add_filter("Comma separated (*.csv)", &["csv"])
            .add_filter("Tab delimited (*.tsv)", &["tsv"])
            .save_file()
        {
            self.save_to(p);
        }
    }

    fn save_to(&mut self, p: PathBuf) {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let r = if ext == "csv" || ext == "tsv" || ext == "txt" { data::save_csv(&self.ds, &p, false) } else { data::save_native(&self.ds, &p) };
        match r {
            Ok(()) => {
                self.dirty = false;
                self.ds_title = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                self.path = Some(p.clone());
                self.remember_recent(&p);
                self.status = format!("Saved {}.", p.display());
            }
            Err(e) => self.message = Some(("Couldn't save".into(), e)),
        }
    }

    fn export_csv(&mut self, labels: bool) {
        if let Some(p) = rfd::FileDialog::new().set_title("Export to CSV").set_file_name(format!("{}.csv", self.ds_title)).add_filter("Comma separated (*.csv)", &["csv"]).save_file() {
            match data::save_csv(&self.ds, &p, labels) {
                Ok(()) => self.status = format!("Exported {}.", p.display()),
                Err(e) => self.message = Some(("Couldn't export".into(), e)),
            }
        }
    }

    // ------------------------------------------------------------ commands

    pub fn open_dialog(&mut self, k: Kind) {
        if self.ds.vars.is_empty() {
            self.message = Some(("No data".into(), "There are no variables yet. Type data into the Data View, open a file, or use File > Open Sample Dataset.".into()));
            return;
        }
        let mut d = match &self.last_dialog {
            Some(ld) if ld.kind == k => ld.clone(),
            _ => Dialog::new(k),
        };
        d.error = None;
        // preselect the selected column, like a helpful shortcut
        if self.sel_mode == SelMode::Col && self.sel.1 < self.ds.vars.len() && d.boxes.iter().all(|b| b.vars.is_empty()) {
            d.src_sel = Some(self.sel.1);
        }
        self.dialog = Some(d);
    }

    pub fn paste_syntax(&mut self, c: &Command) {
        if !self.syn_text.is_empty() && !self.syn_text.ends_with('\n') {
            self.syn_text.push('\n');
        }
        self.syn_text.push_str(&c.to_syntax());
        self.syn_text.push('\n');
        self.syn_open = true;
        self.syn_focus = true;
    }

    fn add_block(&mut self, name: &str, syntax: String, parts: Vec<Part>) {
        let id = self.next_id;
        self.next_id += 1;
        self.out.push(Block { id, name: name.to_string(), syntax, parts });
        let i = self.out.len() - 1;
        self.out_sel = Some((i, None));
        self.out_scroll = Some((i, None));
        self.out_open = true;
        self.out_focus = true;
    }

    pub fn run(&mut self, cmd: Command) {
        let syntax = cmd.to_syntax();
        let before = if cmd.changes_data() { Some(self.ds.clone()) } else { None };
        let t0 = std::time::Instant::now();
        let res = self.exec(&cmd);
        match res {
            Ok(parts) => {
                if let Some(b) = before {
                    self.undo.push(b);
                    self.redo.clear();
                    self.dirty = true;
                }
                self.status = format!("{} finished in {:.0} ms.", cmd.title(), t0.elapsed().as_secs_f64() * 1000.0);
                self.add_block(cmd.title(), syntax, parts);
            }
            Err(e) => {
                if let Some(b) = before {
                    self.ds = b;
                }
                self.status = format!("{} stopped: {}", cmd.title(), e);
                self.add_block(cmd.title(), syntax, vec![Part::Warn(e)]);
            }
        }
        self.bar_for = (usize::MAX, usize::MAX, usize::MAX);
    }

    pub fn run_script(&mut self, text: &str) {
        let cmds = syntax::split_commands(text);
        if cmds.is_empty() {
            self.status = "Nothing to run.".into();
            return;
        }
        for (_, c) in cmds {
            match syntax::parse_command(&c, &self.ds) {
                Ok(Some(cmd)) => self.run(cmd),
                Ok(None) => {}
                Err(e) => {
                    let first = c.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
                    self.add_block("Syntax Error", format!("{}.", c), vec![Part::Warn(format!("{} — {}", first, e))]);
                }
            }
        }
    }

    fn exec(&mut self, cmd: &Command) -> Result<Vec<Part>, String> {
        fn ix(ds: &Dataset, names: &[String]) -> Result<Vec<usize>, String> {
            names.iter().map(|n| ds.var_index(n).ok_or(format!("Variable '{}' doesn't exist", n))).collect()
        }
        fn i1(ds: &Dataset, n: &str) -> Result<usize, String> {
            ds.var_index(n).ok_or(format!("Variable '{}' doesn't exist", n))
        }
        let ds = &self.ds;
        match cmd {
            Command::Descriptives { vars, stats, save } => {
                let v = ix(ds, vars)?;
                stats::descriptives(&mut self.ds, &v, stats, *save)
            }
            Command::Frequencies { vars, table, stats, chart, normal } => stats::frequencies(ds, &ix(ds, vars)?, *table, stats, chart, *normal),
            Command::Crosstabs { rows, cols, cells, chisq, phi } => {
                let mut parts = vec![];
                for &r in &ix(ds, rows)? {
                    for &c in &ix(ds, cols)? {
                        parts.extend(stats::crosstabs(ds, r, c, cells, *chisq, *phi)?);
                    }
                }
                Ok(parts)
            }
            Command::Means { deps, by } => stats::means(ds, &ix(ds, deps)?, i1(ds, by)?),
            Command::TTestOne { vars, value } => stats::ttest_one(ds, &ix(ds, vars)?, *value),
            Command::TTestInd { vars, group, g1, g2 } => stats::ttest_ind(ds, &ix(ds, vars)?, i1(ds, group)?, g1, g2),
            Command::TTestPaired { pairs } => {
                let p: Result<Vec<(usize, usize)>, String> = pairs.iter().map(|(a, b)| Ok((i1(ds, a)?, i1(ds, b)?))).collect();
                stats::ttest_paired(ds, &p?)
            }
            Command::Oneway { deps, factor, desc, homog, bonf } => stats::oneway(ds, &ix(ds, deps)?, i1(ds, factor)?, *desc, *homog, *bonf),
            Command::Correlations { vars, spearman, flag } => stats::correlations(ds, &ix(ds, vars)?, *spearman, *flag),
            Command::Regression { dep, ind, desc } => stats::regression(ds, i1(ds, dep)?, &ix(ds, ind)?, *desc),
            Command::Reliability { vars, item_total } => stats::reliability(ds, &ix(ds, vars)?, *item_total),
            Command::MannWhitney { vars, group, g1, g2 } => stats::mann_whitney(ds, &ix(ds, vars)?, i1(ds, group)?, g1, g2),
            Command::ChiSquare { vars } => stats::chisq_gof(ds, &ix(ds, vars)?),
            Command::Histogram { var, normal } => stats::histogram(ds, i1(ds, var)?, *normal),
            Command::Bar { cat, mean_of } => {
                let m = match mean_of {
                    Some(m) => Some(i1(ds, m)?),
                    None => None,
                };
                stats::bar_chart(ds, i1(ds, cat)?, m)
            }
            Command::Scatter { x, y, fit } => stats::scatter(ds, i1(ds, x)?, i1(ds, y)?, *fit),
            Command::Boxplot { var, by } => {
                let b = match by {
                    Some(b) => Some(i1(ds, b)?),
                    None => None,
                };
                stats::boxplot(ds, i1(ds, var)?, b)
            }
            Command::Pie { cat } => stats::pie(ds, i1(ds, cat)?),
            Command::Display { vars } => Ok(stats::dictionary(ds, &ix(ds, vars)?)),
            Command::Compute { target, expr } => {
                let e = expr::parse(expr, ds)?;
                let is_str = e.is_string(ds);
                let mut cx = expr::Ctx::new();
                let vals: Vec<expr::Val> = (0..ds.ncases).map(|r| e.eval(ds, r, &mut cx)).collect();
                let ti = match self.ds.var_index(target) {
                    Some(i) => {
                        if self.ds.vars[i].is_numeric() == is_str {
                            return Err(format!("{} is a {} variable but the expression gives {}.", target, self.ds.vars[i].type_name().to_lowercase(), if is_str { "text" } else { "numbers" }));
                        }
                        i
                    }
                    None => {
                        let name = data::valid_name(target);
                        let v = if is_str { data::Variable::new_string(&name, self.ds.ncases) } else { data::Variable::new_numeric(&name, self.ds.ncases) };
                        self.ds.add_var(v)
                    }
                };
                let mut valid = 0;
                for (r, v) in vals.into_iter().enumerate() {
                    match v {
                        expr::Val::Num(x) => {
                            if !x.is_nan() {
                                valid += 1;
                            }
                            self.ds.vars[ti].set_num(r, x);
                        }
                        expr::Val::Str(s) => {
                            valid += 1;
                            self.ds.vars[ti].set_text(r, &s);
                        }
                    }
                }
                Ok(vec![Part::Text(format!("{} computed for {} cases ({} with a valid result).", self.ds.vars[ti].name, self.ds.ncases, valid))])
            }
            Command::Sort { keys } => {
                let k: Result<Vec<(usize, bool)>, String> = keys.iter().map(|(n, a)| Ok((i1(ds, n)?, *a))).collect();
                let k = k?;
                self.ds.sort_by(&k);
                Ok(vec![Part::Text(format!("Cases sorted by {}.", keys.iter().map(|(n, a)| format!("{} ({})", n, if *a { "ascending" } else { "descending" })).collect::<Vec<_>>().join(", ")))])
            }
            Command::Filter { expr } => match expr {
                None => {
                    self.ds.filter = None;
                    self.ds.filter_expr.clear();
                    Ok(vec![Part::Text(format!("All {} cases are selected.", self.ds.ncases))])
                }
                Some(src) => {
                    let e = expr::parse(src, ds)?;
                    let mut cx = expr::Ctx::new();
                    let f: Vec<bool> = (0..ds.ncases).map(|r| e.eval(ds, r, &mut cx).truthy()).collect();
                    let n = f.iter().filter(|x| **x).count();
                    self.ds.filter = Some(f);
                    self.ds.filter_expr = src.clone();
                    Ok(vec![Part::Text(format!("{} of {} cases selected (filter: {}). Unselected cases are crossed out in the Data View.", n, self.ds.ncases, src))])
                }
            },
        }
    }

    // ------------------------------------------------------------ menus

    pub fn analysis_menus(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Analyze", |ui| {
            ui.menu_button("Descriptive Statistics", |ui| {
                for (k, l) in [(Kind::Frequencies, "Frequencies..."), (Kind::Descriptives, "Descriptives..."), (Kind::Crosstabs, "Crosstabs...")] {
                    if ui.button(l).clicked() {
                        self.open_dialog(k);
                        ui.close_menu();
                    }
                }
            });
            ui.menu_button("Compare Means", |ui| {
                for (k, l) in [(Kind::Means, "Means..."), (Kind::TOne, "One-Sample T Test..."), (Kind::TInd, "Independent-Samples T Test..."), (Kind::TPaired, "Paired-Samples T Test..."), (Kind::Oneway, "One-Way ANOVA...")] {
                    if ui.button(l).clicked() {
                        self.open_dialog(k);
                        ui.close_menu();
                    }
                }
            });
            ui.menu_button("Correlate", |ui| {
                if ui.button("Bivariate...").clicked() {
                    self.open_dialog(Kind::Correlations);
                    ui.close_menu();
                }
            });
            ui.menu_button("Regression", |ui| {
                if ui.button("Linear...").clicked() {
                    self.open_dialog(Kind::Regression);
                    ui.close_menu();
                }
            });
            ui.menu_button("Scale", |ui| {
                if ui.button("Reliability Analysis...").clicked() {
                    self.open_dialog(Kind::Reliability);
                    ui.close_menu();
                }
            });
            ui.menu_button("Nonparametric Tests", |ui| {
                if ui.button("Chi-square...").clicked() {
                    self.open_dialog(Kind::ChiSquare);
                    ui.close_menu();
                }
                if ui.button("2 Independent Samples (Mann-Whitney)...").clicked() {
                    self.open_dialog(Kind::MannWhitney);
                    ui.close_menu();
                }
            });
        });
        ui.menu_button("Graphs", |ui| {
            for (k, l) in [(Kind::Bar, "Bar..."), (Kind::Histogram, "Histogram..."), (Kind::Scatter, "Scatter/Dot..."), (Kind::Boxplot, "Boxplot..."), (Kind::Pie, "Pie...")] {
                if ui.button(l).clicked() {
                    self.open_dialog(k);
                    ui.close_menu();
                }
            }
        });
    }

    fn window_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Window", |ui| {
            if ui.button(format!("{} [DataSet1] - Data Editor", self.ds_title)).clicked() {
                ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                ui.close_menu();
            }
            if ui.button("Output Viewer").clicked() {
                self.out_open = true;
                self.out_focus = true;
                ui.close_menu();
            }
            if ui.button("Syntax Editor").clicked() {
                self.syn_open = true;
                self.syn_focus = true;
                ui.close_menu();
            }
        });
    }

    fn main_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                ui.menu_button("New", |ui| {
                    if ui.button("Data").clicked() {
                        self.guarded(Pending::New);
                        ui.close_menu();
                    }
                    if ui.button("Syntax").clicked() {
                        self.syn_open = true;
                        self.syn_focus = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Open", |ui| {
                    if ui.button("Data...").clicked() {
                        ui.close_menu();
                        self.guarded(Pending::Open);
                    }
                    if ui.button("Syntax...").clicked() {
                        ui.close_menu();
                        self.open_syntax();
                    }
                });
                if ui.button("Open Sample Dataset").clicked() {
                    ui.close_menu();
                    self.guarded(Pending::Sample);
                }
                ui.separator();
                if ui.add(egui::Button::new("Save").shortcut_text("Ctrl+S")).clicked() {
                    ui.close_menu();
                    self.save();
                }
                if ui.button("Save As...").clicked() {
                    ui.close_menu();
                    self.save_as();
                }
                ui.menu_button("Export", |ui| {
                    if ui.button("CSV (values)...").clicked() {
                        ui.close_menu();
                        self.export_csv(false);
                    }
                    if ui.button("CSV (value labels)...").clicked() {
                        ui.close_menu();
                        self.export_csv(true);
                    }
                });
                ui.separator();
                ui.menu_button("Recently Used Data", |ui| {
                    if self.recent.is_empty() {
                        ui.label(RichText::new("(none yet)").weak());
                    }
                    for p in self.recent.clone() {
                        if ui.button(p.display().to_string()).clicked() {
                            ui.close_menu();
                            self.guarded(Pending::OpenPath(p));
                        }
                    }
                });
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.close_menu();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo").shortcut_text("Ctrl+Z")).clicked() {
                    self.do_undo();
                    ui.close_menu();
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo").shortcut_text("Ctrl+Y")).clicked() {
                    self.do_redo();
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Copy").shortcut_text("Ctrl+C")).clicked() {
                    let t = self.selection_text();
                    ui.ctx().copy_text(t);
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Clear").shortcut_text("Del")).clicked() {
                    self.clear_selection();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Insert Variable").clicked() {
                    self.insert_variable();
                    ui.close_menu();
                }
                if ui.button("Insert Cases").clicked() {
                    self.insert_case();
                    ui.close_menu();
                }
                ui.separator();
                if ui.add(egui::Button::new("Find...").shortcut_text("Ctrl+F")).clicked() {
                    self.find = Some(FindDlg { text: String::new(), whole: false, msg: String::new() });
                    ui.close_menu();
                }
                if ui.button("Go to Case...").clicked() {
                    self.goto = Some(GotoDlg { case: true, text: String::new(), var: 0 });
                    ui.close_menu();
                }
                if ui.button("Go to Variable...").clicked() {
                    self.goto = Some(GotoDlg { case: false, text: String::new(), var: self.sel.1.min(self.ds.vars.len().saturating_sub(1)) });
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.labels_on, "Value Labels");
                ui.checkbox(&mut self.grid_lines, "Grid Lines");
                ui.separator();
                if ui.button("Data View").clicked() {
                    self.view = View::Data;
                    ui.close_menu();
                }
                if ui.button("Variable View").clicked() {
                    self.view = View::Variables;
                    ui.close_menu();
                }
                ui.separator();
                let z = ui.ctx().zoom_factor();
                if ui.add(egui::Button::new("Zoom In").shortcut_text("Ctrl +")).clicked() {
                    ui.ctx().set_zoom_factor((z + 0.1).min(2.0));
                }
                if ui.add(egui::Button::new("Zoom Out").shortcut_text("Ctrl -")).clicked() {
                    ui.ctx().set_zoom_factor((z - 0.1).max(0.6));
                }
                if ui.button("Actual Size").clicked() {
                    ui.ctx().set_zoom_factor(1.0);
                }
            });
            ui.menu_button("Data", |ui| {
                if ui.button("Define Variable Properties...").clicked() {
                    self.view = View::Variables;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Insert Variable").clicked() {
                    self.insert_variable();
                    ui.close_menu();
                }
                if ui.button("Insert Cases").clicked() {
                    self.insert_case();
                    ui.close_menu();
                }
                if ui.button("Go to Case...").clicked() {
                    self.goto = Some(GotoDlg { case: true, text: String::new(), var: 0 });
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Sort Cases...").clicked() {
                    self.open_dialog(Kind::Sort);
                    ui.close_menu();
                }
                if ui.button("Select Cases...").clicked() {
                    if self.ds.vars.is_empty() {
                        self.open_dialog(Kind::Sort);
                    } else {
                        self.select = Some(SelectDlg { use_if: self.ds.filter.is_some(), expr: self.ds.filter_expr.clone(), error: None });
                    }
                    ui.close_menu();
                }
                if ui.add_enabled(self.ds.filter.is_some(), egui::Button::new("Use All Cases")).clicked() {
                    self.run(Command::Filter { expr: None });
                    ui.close_menu();
                }
            });
            ui.menu_button("Transform", |ui| {
                if ui.button("Compute Variable...").clicked() {
                    if self.ds.vars.is_empty() {
                        self.open_dialog(Kind::Sort);
                    } else {
                        self.compute = Some(ComputeDlg::default());
                    }
                    ui.close_menu();
                }
                if ui.button("Standardize (z-scores)...").clicked() {
                    self.open_dialog(Kind::Descriptives);
                    if let Some(d) = &mut self.dialog {
                        if let Some(f) = d.flags.iter_mut().find(|(n, _)| n.starts_with("Save")) {
                            f.1 = true;
                        }
                    }
                    ui.close_menu();
                }
            });
            self.analysis_menus(ui);
            ui.menu_button("Utilities", |ui| {
                if ui.button("Variables...").clicked() {
                    self.open_dialog(Kind::Display);
                    ui.close_menu();
                }
                if ui.button("Run Syntax Editor").clicked() {
                    self.syn_open = true;
                    self.syn_focus = true;
                    ui.close_menu();
                }
            });
            self.window_menu(ui);
            ui.menu_button("Help", |ui| {
                if ui.button("Topics").clicked() {
                    self.help = true;
                    ui.close_menu();
                }
                if ui.button("Welcome Dialog").clicked() {
                    self.welcome = true;
                    ui.close_menu();
                }
                if ui.button(format!("About {}", APP_NAME)).clicked() {
                    self.about = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn open_syntax(&mut self) {
        if let Some(p) = rfd::FileDialog::new().set_title("Open Syntax").add_filter("Syntax (*.sds, *.sps, *.txt)", &["sds", "sps", "txt"]).pick_file() {
            match std::fs::read_to_string(&p) {
                Ok(t) => {
                    self.syn_text = t;
                    self.syn_path = Some(p);
                    self.syn_open = true;
                    self.syn_focus = true;
                }
                Err(e) => self.message = Some(("Couldn't open the syntax file".into(), e.to_string())),
            }
        }
    }

    // ------------------------------------------------------------ toolbar

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            if views::tool(ui, views::Icon::Open, "Open data document", true).clicked() {
                self.guarded(Pending::Open);
            }
            if views::tool(ui, views::Icon::Save, "Save this document", true).clicked() {
                self.save();
            }
            if views::tool(ui, views::Icon::Export, "Export to CSV", !self.ds.vars.is_empty()).clicked() {
                self.export_csv(false);
            }
            if views::tool(ui, views::Icon::Recall, "Recall recently used dialogs", self.last_dialog.is_some()).clicked() {
                if let Some(d) = self.last_dialog.clone() {
                    self.dialog = Some(d);
                }
            }
            views::tool_sep(ui);
            if views::tool(ui, views::Icon::Undo, "Undo a user action", !self.undo.is_empty()).clicked() {
                self.do_undo();
            }
            if views::tool(ui, views::Icon::Redo, "Redo a user action", !self.redo.is_empty()).clicked() {
                self.do_redo();
            }
            views::tool_sep(ui);
            if views::tool(ui, views::Icon::GotoCase, "Go to case", true).clicked() {
                self.goto = Some(GotoDlg { case: true, text: String::new(), var: 0 });
            }
            if views::tool(ui, views::Icon::GotoVar, "Go to variable", !self.ds.vars.is_empty()).clicked() {
                self.goto = Some(GotoDlg { case: false, text: String::new(), var: self.sel.1.min(self.ds.vars.len().saturating_sub(1)) });
            }
            if views::tool(ui, views::Icon::Variables, "Variables", !self.ds.vars.is_empty()).clicked() {
                self.open_dialog(Kind::Display);
            }
            if views::tool(ui, views::Icon::Find, "Find", true).clicked() {
                self.find = Some(FindDlg { text: String::new(), whole: false, msg: String::new() });
            }
            views::tool_sep(ui);
            if views::tool(ui, views::Icon::InsertCase, "Insert cases", true).clicked() {
                self.insert_case();
            }
            if views::tool(ui, views::Icon::InsertVar, "Insert variable", true).clicked() {
                self.insert_variable();
            }
            if views::tool(ui, views::Icon::Select, "Select cases", !self.ds.vars.is_empty()).clicked() {
                self.select = Some(SelectDlg { use_if: self.ds.filter.is_some(), expr: self.ds.filter_expr.clone(), error: None });
            }
            let lab = views::tool(ui, views::Icon::ValueLabels, "Value labels", true);
            if self.labels_on {
                ui.painter().rect_stroke(lab.rect.shrink(1.0), 2.0, Stroke::new(1.0, Color32::from_rgb(60, 110, 190)), egui::StrokeKind::Inside);
            }
            if lab.clicked() {
                self.labels_on = !self.labels_on;
            }
            views::tool_sep(ui);
            if views::tool(ui, views::Icon::Chart, "Chart: histogram", !self.ds.vars.is_empty()).clicked() {
                self.open_dialog(Kind::Histogram);
            }
            if views::tool(ui, views::Icon::Stats, "Descriptive statistics", !self.ds.vars.is_empty()).clicked() {
                self.open_dialog(Kind::Descriptives);
            }
            if views::tool(ui, views::Icon::Output, "Show the output viewer", true).clicked() {
                self.out_open = true;
                self.out_focus = true;
            }
            if views::tool(ui, views::Icon::Syntax, "Show the syntax editor", true).clicked() {
                self.syn_open = true;
                self.syn_focus = true;
            }
        });
    }

    // ------------------------------------------------------------ small windows

    fn small_windows(&mut self, ctx: &egui::Context) {
        if let Some((title, text)) = self.message.clone() {
            let mut close = false;
            egui::Window::new(title).id(egui::Id::new("message")).collapsible(false).resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
                ui.set_max_width(420.0);
                ui.label(text);
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    if ui.add_sized([80.0, 26.0], egui::Button::new("OK")).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
                        close = true;
                    }
                });
            });
            if close {
                self.message = None;
            }
        }
        if let Some(p) = self.pending.clone() {
            let mut choice: Option<u8> = None;
            egui::Window::new(APP_NAME).id(egui::Id::new("pending")).collapsible(false).resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 14.0, Color32::from_rgb(50, 110, 200));
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(20.0), Color32::WHITE);
                    ui.label(format!("Save the contents of the data editor to {}?", self.ds_title));
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add_sized([80.0, 26.0], egui::Button::new("Yes")).clicked() {
                        choice = Some(0);
                    }
                    if ui.add_sized([80.0, 26.0], egui::Button::new("No")).clicked() {
                        choice = Some(1);
                    }
                    if ui.add_sized([80.0, 26.0], egui::Button::new("Cancel")).clicked() {
                        choice = Some(2);
                    }
                });
            });
            match choice {
                Some(0) => {
                    self.save();
                    self.pending = None;
                    if !self.dirty {
                        let exit = matches!(p, Pending::Exit);
                        self.do_pending(p);
                        if exit {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
                Some(1) => {
                    self.pending = None;
                    self.dirty = false;
                    let exit = matches!(p, Pending::Exit);
                    self.do_pending(p);
                    if exit {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Some(_) => self.pending = None,
                None => {}
            }
        }
        if self.about {
            let mut open = true;
            egui::Window::new(format!("About {}", APP_NAME)).open(&mut open).collapsible(false).resizable(false).pivot(egui::Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                    views::paint_logo(ui.painter(), r);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(APP_NAME).size(22.0).strong());
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.label("A lightweight statistics workbench.");
                    });
                });
                ui.add_space(6.0);
                ui.label("Data editor, variable dictionary, descriptive statistics, crosstabs, t tests, ANOVA, correlation, regression, reliability, nonparametric tests, charts, syntax and an output viewer, in one small program.");
            });
            self.about = open;
        }
        if self.help {
            let mut open = true;
            egui::Window::new("Help Topics").open(&mut open).default_size([560.0, 460.0]).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (h, t) in views::HELP {
                        ui.label(RichText::new(*h).strong().size(14.0));
                        ui.label(*t);
                        ui.add_space(8.0);
                    }
                });
            });
            self.help = open;
        }
    }

    fn welcome_ui(&mut self, ctx: &egui::Context) {
        if !self.welcome {
            return;
        }
        let mut open = true;
        let mut act: Option<Pending> = None;
        let mut new_syntax = false;
        egui::Window::new(format!("Welcome to {}", APP_NAME))
            .id(egui::Id::new("welcome"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .fixed_size([620.0, 360.0])
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(300.0);
                        ui.label(RichText::new("New Files:").strong());
                        if views::welcome_item(ui, views::Icon::NewData, "New Dataset", "An empty data sheet").clicked() {
                            act = Some(Pending::New);
                        }
                        if views::welcome_item(ui, views::Icon::Syntax, "New Syntax", "Write and run commands").clicked() {
                            new_syntax = true;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("Recent Files:").strong());
                        egui::Frame::default().fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_gray(200))).inner_margin(egui::Margin::same(4)).show(ui, |ui| {
                            ui.set_width(292.0);
                            ui.set_height(120.0);
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                if views::welcome_item(ui, views::Icon::Open, "Open another file...", "CSV, tab-delimited or StatDesk data").clicked() {
                                    act = Some(Pending::Open);
                                }
                                for p in self.recent.clone() {
                                    let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                    if views::welcome_item(ui, views::Icon::NewData, &name, &p.display().to_string()).clicked() {
                                        act = Some(Pending::OpenPath(p));
                                    }
                                }
                            });
                        });
                    });
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(290.0, 120.0), Sense::hover());
                        views::paint_welcome_art(ui.painter(), r);
                        ui.add_space(6.0);
                        ui.label(RichText::new("Get started").strong());
                        ui.label("Try every analysis on a made-up workplace survey with 150 respondents, or type your own data straight into the sheet.");
                        ui.add_space(4.0);
                        if ui.add_sized([200.0, 28.0], egui::Button::new("Open the sample dataset")).clicked() {
                            act = Some(Pending::Sample);
                        }
                    });
                });
                ui.add_space(6.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.no_welcome, "Don't show this dialog in the future");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized([80.0, 26.0], egui::Button::new("Close")).clicked() {
                            self.welcome = false;
                        }
                    });
                });
            });
        if new_syntax {
            self.syn_open = true;
            self.syn_focus = true;
            self.welcome = false;
        }
        if let Some(a) = act {
            self.welcome = false;
            self.guarded(a);
        }
        if !open {
            self.welcome = false;
        }
        if !self.welcome && self.shot.is_none() {
            if let Some(dir) = config_dir() {
                let f = dir.join("no-welcome");
                if self.no_welcome {
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(f, "1");
                } else {
                    let _ = std::fs::remove_file(f);
                }
            }
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(mut d) = self.dialog.take() {
            match d.ui(ctx, &self.ds) {
                Action::None => self.dialog = Some(d),
                Action::Cancel => {}
                Action::Ok(c) => {
                    self.last_dialog = Some(d);
                    self.run(c);
                }
                Action::Paste(c) => {
                    self.last_dialog = Some(d);
                    self.paste_syntax(&c);
                }
            }
        }
        if let Some(mut d) = self.compute.take() {
            match dialogs::compute_ui(&mut d, ctx, &self.ds) {
                Action::None => self.compute = Some(d),
                Action::Cancel => {}
                Action::Ok(c) => self.run(c),
                Action::Paste(c) => self.paste_syntax(&c),
            }
        }
        if let Some(mut d) = self.select.take() {
            match dialogs::select_ui(&mut d, ctx, &self.ds) {
                Action::None => self.select = Some(d),
                Action::Cancel => {}
                Action::Ok(c) => self.run(c),
                Action::Paste(c) => self.paste_syntax(&c),
            }
        }
        self.value_labels_ui(ctx);
        self.missing_ui(ctx);
        self.goto_ui(ctx);
        self.find_ui(ctx);
    }

    pub fn modal_open(&self) -> bool {
        self.dialog.is_some() || self.compute.is_some() || self.select.is_some() || self.vlabels.is_some() || self.missing_dlg.is_some() || self.goto.is_some() || self.find.is_some() || self.message.is_some() || self.pending.is_some() || self.welcome
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&self.status).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("Unicode:ON").size(12.0));
                ui.separator();
                if self.ds.filter.is_some() {
                    ui.label(RichText::new("Filter On").size(12.0).color(Color32::from_rgb(170, 90, 0)));
                    ui.separator();
                }
                ui.label(RichText::new(format!("Cases: {}   Variables: {}", self.ds.ncases, self.ds.vars.len())).size(12.0));
                ui.separator();
                ui.label(RichText::new(format!("{} Processor is ready", APP_NAME)).size(12.0));
                ui.separator();
            });
        });
    }

    fn view_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (v, name) in [(View::Data, "Data View"), (View::Variables, "Variable View")] {
                let sel = self.view == v;
                let (r, resp) = ui.allocate_exact_size(Vec2::new(110.0, 24.0), Sense::click());
                let p = ui.painter();
                let fill = if sel { Color32::WHITE } else if resp.hovered() { Color32::from_rgb(232, 238, 246) } else { Color32::from_rgb(226, 229, 233) };
                p.rect_filled(r, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, fill);
                p.line_segment([Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.max.y)], Stroke::new(1.0, Color32::from_gray(190)));
                if sel {
                    p.line_segment([Pos2::new(r.min.x, r.max.y - 1.0), Pos2::new(r.max.x, r.max.y - 1.0)], Stroke::new(2.0, Color32::from_rgb(40, 100, 190)));
                }
                p.text(r.center(), egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(13.0), if sel { Color32::from_rgb(20, 60, 130) } else { Color32::from_gray(60) });
                if resp.clicked() {
                    if let Some(_) = self.editing {
                        self.commit_edit();
                    }
                    self.view = v;
                }
            }
        });
    }

    fn take_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot.clone() else { return };
        self.frames += 1;
        ctx.request_repaint();
        if self.frames == 120 && !self.shot_sent {
            self.shot_sent = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        if let Some(img) = screenshot_event(ctx) {
            if let Err(e) = save_png(&path, &img) {
                eprintln!("screenshot failed: {}", e);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.frames > 600 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn screenshot_event(ctx: &egui::Context) -> Option<std::sync::Arc<egui::ColorImage>> {
    ctx.input(|i| {
        i.raw.events.iter().find_map(|e| match e {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    })
}

pub fn save_png(path: &std::path::Path, img: &egui::ColorImage) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let w = std::io::BufWriter::new(f);
    let mut enc = png::Encoder::new(w, img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    let mut buf = Vec::with_capacity(img.pixels.len() * 4);
    for p in &img.pixels {
        buf.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
    }
    wr.write_image_data(&buf).map_err(|e| e.to_string())
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // window title
        let title = format!("{}{} [DataSet1] - {} Data Editor", if self.dirty { "*" } else { "" }, self.ds_title, APP_NAME);
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        // closing with unsaved data
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && !self.allow_close && self.shot.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Exit);
        }
        // dropped files
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if let Some(p) = dropped.into_iter().next() {
            let is_syntax = p.extension().map(|e| e.eq_ignore_ascii_case("sps") || e.eq_ignore_ascii_case("sds")).unwrap_or(false);
            if is_syntax {
                if let Ok(t) = std::fs::read_to_string(&p) {
                    self.syn_text = t;
                    self.syn_path = Some(p);
                    self.syn_open = true;
                }
            } else {
                self.guarded(Pending::OpenPath(p));
            }
        }
        // global shortcuts
        let (ctrl_s, ctrl_o, ctrl_z, ctrl_y, ctrl_f, zin, zout) = ctx.input(|i| {
            let c = i.modifiers.command;
            (
                c && i.key_pressed(egui::Key::S),
                c && i.key_pressed(egui::Key::O),
                c && i.key_pressed(egui::Key::Z),
                c && i.key_pressed(egui::Key::Y),
                c && i.key_pressed(egui::Key::F),
                c && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)),
                c && i.key_pressed(egui::Key::Minus),
            )
        });
        if ctrl_s {
            self.save();
        }
        if ctrl_o {
            self.guarded(Pending::Open);
        }
        if !ctx.wants_keyboard_input() {
            if ctrl_z {
                self.do_undo();
            }
            if ctrl_y {
                self.do_redo();
            }
        }
        if ctrl_f {
            self.find = Some(FindDlg { text: String::new(), whole: false, msg: String::new() });
        }
        if zin {
            ctx.set_zoom_factor((ctx.zoom_factor() + 0.1).min(2.0));
        }
        if zout {
            ctx.set_zoom_factor((ctx.zoom_factor() - 0.1).max(0.6));
        }

        if self.shot_output {
            // test mode: draw the output viewer in the main window so it can be photographed
            self.out_open = false;
            self.output_ui(ctx);
            self.dialogs(ctx);
            self.take_screenshot(ctx);
            return;
        }
        egui::TopBottomPanel::top("menu").frame(egui::Frame::default().fill(Color32::from_rgb(250, 250, 251)).inner_margin(egui::Margin::symmetric(4, 2))).show(ctx, |ui| self.main_menu(ui));
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::default().fill(Color32::from_rgb(243, 245, 248)).inner_margin(egui::Margin::symmetric(6, 3)).stroke(Stroke::new(1.0, Color32::from_gray(215))))
            .show(ctx, |ui| self.toolbar(ui));
        egui::TopBottomPanel::bottom("status").frame(egui::Frame::default().fill(Color32::from_rgb(236, 238, 241)).inner_margin(egui::Margin::symmetric(8, 2))).show(ctx, |ui| self.status_bar(ui));
        egui::TopBottomPanel::bottom("tabs").frame(egui::Frame::default().fill(Color32::from_rgb(244, 245, 247)).inner_margin(egui::Margin { left: 4, right: 4, top: 0, bottom: 2 })).show(ctx, |ui| self.view_tabs(ui));
        if self.view == View::Data {
            egui::TopBottomPanel::top("cellbar").frame(egui::Frame::default().fill(Color32::WHITE).inner_margin(egui::Margin::symmetric(6, 3)).stroke(Stroke::new(1.0, Color32::from_gray(215)))).show(ctx, |ui| self.cell_bar(ui));
        }
        egui::CentralPanel::default().frame(egui::Frame::default().fill(Color32::WHITE)).show(ctx, |ui| match self.view {
            View::Data => self.data_view(ui),
            View::Variables => self.variable_view(ui),
        });
        if self.view == View::Data && !self.modal_open() {
            self.grid_keys(ctx);
        }
        self.dialogs(ctx);
        self.welcome_ui(ctx);
        self.small_windows(ctx);
        if self.out_open {
            self.output_viewport(ctx);
        }
        if self.syn_open {
            self.syntax_viewport(ctx);
        }
        self.take_screenshot(ctx);
    }
}

fn make_icon() -> egui::IconData {
    let n = 32usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) * 4;
            let inside = x >= 1 && x < n - 1 && y >= 1 && y < n - 1;
            let (mut r, mut g, mut b, mut a) = (0u8, 0u8, 0u8, 0u8);
            if inside {
                r = 34;
                g = 92;
                b = 170;
                a = 255;
                let bars = [(6usize, 18usize), (13, 10), (20, 14)];
                for (bx, top) in bars {
                    if x >= bx && x < bx + 5 && y >= top && y < 26 {
                        r = 255;
                        g = 255;
                        b = 255;
                    }
                }
            }
            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = a;
        }
    }
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut shot: Option<PathBuf> = None;
    let mut open: Option<PathBuf> = None;
    let mut i = 1;
    let mut startup = Startup::default();
    while i < args.len() {
        if args[i] == "--screenshot" {
            shot = args.get(i + 1).map(PathBuf::from);
            i += 2;
        } else if args[i] == "--sample" {
            startup.sample = true;
            i += 1;
        } else if args[i] == "--run" {
            startup.run = args.get(i + 1).cloned();
            i += 2;
        } else if args[i] == "--shot-output" {
            startup.shot_output = true;
            i += 1;
        } else if args[i] == "--variable-view" {
            startup.var_view = true;
            i += 1;
        } else if args[i] == "--open-dialog" {
            startup.dialog = args.get(i + 1).cloned();
            i += 2;
        } else {
            open = Some(PathBuf::from(&args[i]));
            i += 1;
        }
    }
    let mut vp = egui::ViewportBuilder::default()
        .with_title(format!("Untitled1 [DataSet1] - {} Data Editor", APP_NAME))
        .with_inner_size([1240.0, 780.0])
        .with_min_inner_size([640.0, 420.0])
        .with_icon(make_icon())
        .with_drag_and_drop(true);
    if shot.is_some() {
        vp = vp.with_active(false);
        #[cfg(target_os = "windows")]
        {
            vp = vp.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions { viewport: vp, ..Default::default() };
    #[cfg(target_os = "macos")]
    if shot.is_some() {
        options.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            b.with_activate_ignoring_other_apps(false);
        }));
    }
    let _ = Rect::NOTHING;
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            let mut app = App::new(cc, shot, open);
            app.apply_startup(startup);
            Ok(Box::new(app))
        }),
    )
}

/// Optional start-up actions (used for demos and automated checks).
#[derive(Default)]
struct Startup {
    sample: bool,
    run: Option<String>,
    shot_output: bool,
    var_view: bool,
    dialog: Option<String>,
}

impl App {
    fn apply_startup(&mut self, s: Startup) {
        if s.sample {
            self.welcome = false;
            self.do_pending(Pending::Sample);
        }
        if let Some(cmds) = s.run {
            self.welcome = false;
            self.run_script(&cmds.replace("\\n", "\n"));
            self.out_focus = false;
        }
        if s.var_view {
            self.view = View::Variables;
        }
        match s.dialog.as_deref() {
            Some("compute") => self.compute = Some(ComputeDlg::default()),
            Some("select") => self.select = Some(SelectDlg::default()),
            Some(name) => {
                let k = [Kind::Descriptives, Kind::Frequencies, Kind::Crosstabs, Kind::TInd, Kind::Oneway, Kind::Regression, Kind::Correlations, Kind::Histogram, Kind::Scatter]
                    .into_iter()
                    .find(|k| k.title().to_ascii_lowercase().starts_with(&name.to_ascii_lowercase()));
                if let Some(k) = k {
                    self.welcome = false;
                    self.open_dialog(k);
                    if let Some(d) = &mut self.dialog {
                        if !self.ds.vars.is_empty() {
                            d.boxes[0].vars = vec![2, 5];
                            d.src_sel = Some(4);
                        }
                    }
                }
            }
            None => {}
        }
        self.shot_output = s.shot_output;
    }
}
