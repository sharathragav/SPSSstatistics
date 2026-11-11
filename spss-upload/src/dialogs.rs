//! Analysis dialogs: pick variables into boxes, set options, then OK or Paste.

use crate::data::{Dataset, Measure};
use crate::syntax::Command;
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Descriptives,
    Frequencies,
    Crosstabs,
    Means,
    TOne,
    TInd,
    TPaired,
    Oneway,
    Correlations,
    Regression,
    Reliability,
    MannWhitney,
    ChiSquare,
    Histogram,
    Bar,
    Scatter,
    Boxplot,
    Pie,
    Sort,
    Display,
}

impl Kind {
    pub fn title(&self) -> &'static str {
        match self {
            Kind::Descriptives => "Descriptives",
            Kind::Frequencies => "Frequencies",
            Kind::Crosstabs => "Crosstabs",
            Kind::Means => "Means",
            Kind::TOne => "One-Sample T Test",
            Kind::TInd => "Independent-Samples T Test",
            Kind::TPaired => "Paired-Samples T Test",
            Kind::Oneway => "One-Way ANOVA",
            Kind::Correlations => "Bivariate Correlations",
            Kind::Regression => "Linear Regression",
            Kind::Reliability => "Reliability Analysis",
            Kind::MannWhitney => "Two-Independent-Samples Tests",
            Kind::ChiSquare => "Chi-square Test",
            Kind::Histogram => "Histogram",
            Kind::Bar => "Simple Bar Chart",
            Kind::Scatter => "Simple Scatterplot",
            Kind::Boxplot => "Boxplot",
            Kind::Pie => "Pie Chart",
            Kind::Sort => "Sort Cases",
            Kind::Display => "Variables",
        }
    }
    pub fn help(&self) -> &'static str {
        match self {
            Kind::Descriptives => "Summary statistics (mean, standard deviation, minimum, maximum and more) for numeric variables. Tick 'Save standardized values' to add z-score variables to the data.",
            Kind::Frequencies => "Counts and percentages for each value of a variable, with optional statistics and charts.",
            Kind::Crosstabs => "A table of counts for two categorical variables, with an optional chi-square test of independence.",
            Kind::Means => "Means, counts and standard deviations of a numeric variable within the groups of another.",
            Kind::TOne => "Tests whether the mean of a variable differs from a fixed test value.",
            Kind::TInd => "Compares the means of two groups. Type the two group values (for example 1 and 2).",
            Kind::TPaired => "Compares two measurements on the same cases. Put the first of each pair in Variable 1 and the second in Variable 2, in the same order.",
            Kind::Oneway => "Compares the means of several groups with an F test; optional descriptives, Levene's test and Bonferroni comparisons.",
            Kind::Correlations => "Pearson or Spearman correlations between every pair of variables, with two-tailed significance.",
            Kind::Regression => "Ordinary least squares regression of one dependent variable on one or more predictors.",
            Kind::Reliability => "Cronbach's alpha for a set of scale items, with item-total statistics.",
            Kind::MannWhitney => "The Mann-Whitney U test compares two groups without assuming normal data.",
            Kind::ChiSquare => "Tests whether all categories of a variable are equally common.",
            Kind::Histogram => "The distribution of a numeric variable, optionally with a normal curve.",
            Kind::Bar => "One bar per category: the number of cases, or the mean of another variable.",
            Kind::Scatter => "Plots two numeric variables against each other, optionally with a fitted line.",
            Kind::Boxplot => "Median, quartiles, whiskers and outliers of a variable, optionally for each group.",
            Kind::Pie => "Shares of each category as slices.",
            Kind::Sort => "Reorders the cases by one or more variables.",
            Kind::Display => "Writes the dictionary (names, labels, formats, value labels) of the chosen variables to the output.",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VBox {
    pub label: &'static str,
    pub vars: Vec<usize>,
    pub max: usize,
}

#[derive(Clone, Debug)]
pub struct Dialog {
    pub kind: Kind,
    pub boxes: Vec<VBox>,
    pub src_sel: Option<usize>,
    pub box_sel: Option<(usize, usize)>,
    pub flags: Vec<(&'static str, bool)>,
    pub choice_label: &'static str,
    pub choices: Vec<&'static str>,
    pub choice: usize,
    pub texts: Vec<(&'static str, String)>,
    pub error: Option<String>,
    pub show_help: bool,
}

pub enum Action {
    None,
    Ok(Command),
    Paste(Command),
    Cancel,
}

fn b(label: &'static str, max: usize) -> VBox {
    VBox { label, vars: vec![], max }
}

impl Dialog {
    pub fn new(kind: Kind) -> Self {
        let mut d = Dialog {
            kind,
            boxes: vec![],
            src_sel: None,
            box_sel: None,
            flags: vec![],
            choice_label: "",
            choices: vec![],
            choice: 0,
            texts: vec![],
            error: None,
            show_help: false,
        };
        match kind {
            Kind::Descriptives => {
                d.boxes = vec![b("Variable(s):", 0)];
                d.flags = vec![
                    ("Save standardized values as variables", false),
                    ("Mean", true),
                    ("Sum", false),
                    ("Std. deviation", true),
                    ("Variance", false),
                    ("Range", false),
                    ("Minimum", true),
                    ("Maximum", true),
                    ("S.E. mean", false),
                    ("Kurtosis", false),
                    ("Skewness", false),
                ];
            }
            Kind::Frequencies => {
                d.boxes = vec![b("Variable(s):", 0)];
                d.flags = vec![
                    ("Display frequency tables", true),
                    ("Mean", false),
                    ("Median", false),
                    ("Mode", false),
                    ("Std. deviation", false),
                    ("Variance", false),
                    ("Range", false),
                    ("Minimum", false),
                    ("Maximum", false),
                    ("Sum", false),
                    ("Quartiles", false),
                    ("Skewness", false),
                    ("Kurtosis", false),
                    ("Normal curve on histograms", true),
                ];
                d.choice_label = "Charts:";
                d.choices = vec!["None", "Bar charts", "Pie charts", "Histograms"];
            }
            Kind::Crosstabs => {
                d.boxes = vec![b("Row(s):", 0), b("Column(s):", 0)];
                d.flags = vec![
                    ("Chi-square", true),
                    ("Phi and Cramer's V", false),
                    ("Observed counts", true),
                    ("Expected counts", false),
                    ("Row percentages", false),
                    ("Column percentages", false),
                    ("Total percentages", false),
                ];
            }
            Kind::Means => d.boxes = vec![b("Dependent List:", 0), b("Independent List:", 1)],
            Kind::TOne => {
                d.boxes = vec![b("Test Variable(s):", 0)];
                d.texts = vec![("Test Value:", "0".into())];
            }
            Kind::TInd => {
                d.boxes = vec![b("Test Variable(s):", 0), b("Grouping Variable:", 1)];
                d.texts = vec![("Group 1:", "1".into()), ("Group 2:", "2".into())];
            }
            Kind::TPaired => d.boxes = vec![b("Variable 1 (one per pair):", 0), b("Variable 2 (same order):", 0)],
            Kind::Oneway => {
                d.boxes = vec![b("Dependent List:", 0), b("Factor:", 1)];
                d.flags = vec![("Descriptive", true), ("Homogeneity of variance test", false), ("Bonferroni post hoc", false)];
            }
            Kind::Correlations => {
                d.boxes = vec![b("Variables:", 0)];
                d.choice_label = "Correlation Coefficients:";
                d.choices = vec!["Pearson", "Spearman"];
                d.flags = vec![("Flag significant correlations", true)];
            }
            Kind::Regression => {
                d.boxes = vec![b("Dependent:", 1), b("Independent(s):", 0)];
                d.flags = vec![("Descriptives", false)];
            }
            Kind::Reliability => {
                d.boxes = vec![b("Items:", 0)];
                d.flags = vec![("Item-total statistics", true)];
            }
            Kind::MannWhitney => {
                d.boxes = vec![b("Test Variable List:", 0), b("Grouping Variable:", 1)];
                d.texts = vec![("Group 1:", "1".into()), ("Group 2:", "2".into())];
            }
            Kind::ChiSquare => d.boxes = vec![b("Test Variable List:", 0)],
            Kind::Histogram => {
                d.boxes = vec![b("Variable:", 1)];
                d.flags = vec![("Display normal curve", true)];
            }
            Kind::Bar => {
                d.boxes = vec![b("Category Axis:", 1), b("Variable (for mean of):", 1)];
                d.choice_label = "Bars Represent:";
                d.choices = vec!["N of cases", "Mean of variable"];
            }
            Kind::Scatter => {
                d.boxes = vec![b("Y Axis:", 1), b("X Axis:", 1)];
                d.flags = vec![("Fit line", true)];
            }
            Kind::Boxplot => d.boxes = vec![b("Variable:", 1), b("Category Axis (optional):", 1)],
            Kind::Pie => d.boxes = vec![b("Define Slices by:", 1)],
            Kind::Sort => {
                d.boxes = vec![b("Sort by:", 0)];
                d.choice_label = "Sort Order:";
                d.choices = vec!["Ascending", "Descending"];
            }
            Kind::Display => d.boxes = vec![b("Variables:", 0)],
        }
        d
    }

    fn flag(&self, name: &str) -> bool {
        self.flags.iter().find(|(n, _)| *n == name).map(|(_, v)| *v).unwrap_or(false)
    }
    fn text(&self, name: &str) -> String {
        self.texts.iter().find(|(n, _)| *n == name).map(|(_, v)| v.trim().to_string()).unwrap_or_default()
    }

    pub fn to_command(&self, ds: &Dataset) -> Result<Command, String> {
        let names = |i: usize| -> Vec<String> { self.boxes[i].vars.iter().filter(|&&v| v < ds.vars.len()).map(|&v| ds.vars[v].name.clone()).collect() };
        let need = |i: usize| -> Result<Vec<String>, String> {
            let n = names(i);
            if n.is_empty() {
                Err(format!("Move at least one variable into '{}'", self.boxes[i].label.trim_end_matches(':')))
            } else {
                Ok(n)
            }
        };
        let one = |i: usize| -> Result<String, String> { need(i).map(|v| v[0].clone()) };
        let opt = |i: usize| -> Option<String> { names(i).into_iter().next() };
        Ok(match self.kind {
            Kind::Descriptives => {
                let map = [("Mean", "MEAN"), ("Sum", "SUM"), ("Std. deviation", "STDDEV"), ("Variance", "VARIANCE"), ("Range", "RANGE"), ("Minimum", "MIN"), ("Maximum", "MAX"), ("S.E. mean", "SEMEAN"), ("Kurtosis", "KURTOSIS"), ("Skewness", "SKEWNESS")];
                let stats: Vec<String> = map.iter().filter(|(f, _)| self.flag(f)).map(|(_, k)| k.to_string()).collect();
                Command::Descriptives { vars: need(0)?, stats, save: self.flag("Save standardized values as variables") }
            }
            Kind::Frequencies => {
                let map = [
                    ("Mean", "MEAN"),
                    ("Median", "MEDIAN"),
                    ("Mode", "MODE"),
                    ("Std. deviation", "STDDEV"),
                    ("Variance", "VARIANCE"),
                    ("Range", "RANGE"),
                    ("Minimum", "MINIMUM"),
                    ("Maximum", "MAXIMUM"),
                    ("Sum", "SUM"),
                    ("Quartiles", "QUARTILES"),
                    ("Skewness", "SKEWNESS"),
                    ("Kurtosis", "KURTOSIS"),
                ];
                let stats: Vec<String> = map.iter().filter(|(f, _)| self.flag(f)).map(|(_, k)| k.to_string()).collect();
                let chart = ["", "BAR", "PIE", "HISTOGRAM"][self.choice.min(3)].to_string();
                Command::Frequencies { vars: need(0)?, table: self.flag("Display frequency tables"), stats, chart, normal: self.flag("Normal curve on histograms") }
            }
            Kind::Crosstabs => {
                let mut cells = vec![];
                for (f, k) in [("Observed counts", "COUNT"), ("Expected counts", "EXPECTED"), ("Row percentages", "ROW"), ("Column percentages", "COLUMN"), ("Total percentages", "TOTAL")] {
                    if self.flag(f) {
                        cells.push(k.to_string());
                    }
                }
                if cells.is_empty() {
                    cells.push("COUNT".into());
                }
                Command::Crosstabs { rows: need(0)?, cols: need(1)?, cells, chisq: self.flag("Chi-square"), phi: self.flag("Phi and Cramer's V") }
            }
            Kind::Means => Command::Means { deps: need(0)?, by: one(1)? },
            Kind::TOne => {
                let v: f64 = self.text("Test Value:").parse().map_err(|_| "The test value must be a number".to_string())?;
                Command::TTestOne { vars: need(0)?, value: v }
            }
            Kind::TInd => {
                let (g1, g2) = (self.text("Group 1:"), self.text("Group 2:"));
                if g1.is_empty() || g2.is_empty() {
                    return Err("Type both group values".into());
                }
                Command::TTestInd { vars: need(0)?, group: one(1)?, g1, g2 }
            }
            Kind::TPaired => {
                let a = need(0)?;
                let bb = need(1)?;
                if a.len() != bb.len() {
                    return Err("Both boxes need the same number of variables (one pair per row)".into());
                }
                Command::TTestPaired { pairs: a.into_iter().zip(bb).collect() }
            }
            Kind::Oneway => Command::Oneway { deps: need(0)?, factor: one(1)?, desc: self.flag("Descriptive"), homog: self.flag("Homogeneity of variance test"), bonf: self.flag("Bonferroni post hoc") },
            Kind::Correlations => {
                let v = need(0)?;
                if v.len() < 2 {
                    return Err("Choose at least two variables".into());
                }
                Command::Correlations { vars: v, spearman: self.choice == 1, flag: self.flag("Flag significant correlations") }
            }
            Kind::Regression => Command::Regression { dep: one(0)?, ind: need(1)?, desc: self.flag("Descriptives") },
            Kind::Reliability => Command::Reliability { vars: need(0)?, item_total: self.flag("Item-total statistics") },
            Kind::MannWhitney => {
                let (g1, g2) = (self.text("Group 1:"), self.text("Group 2:"));
                if g1.is_empty() || g2.is_empty() {
                    return Err("Type both group values".into());
                }
                Command::MannWhitney { vars: need(0)?, group: one(1)?, g1, g2 }
            }
            Kind::ChiSquare => Command::ChiSquare { vars: need(0)? },
            Kind::Histogram => Command::Histogram { var: one(0)?, normal: self.flag("Display normal curve") },
            Kind::Bar => {
                let cat = one(0)?;
                let mean_of = if self.choice == 1 { Some(one(1)?) } else { None };
                Command::Bar { cat, mean_of }
            }
            Kind::Scatter => Command::Scatter { y: one(0)?, x: one(1)?, fit: self.flag("Fit line") },
            Kind::Boxplot => Command::Boxplot { var: one(0)?, by: opt(1) },
            Kind::Pie => Command::Pie { cat: one(0)? },
            Kind::Sort => Command::Sort { keys: need(0)?.into_iter().map(|n| (n, self.choice == 0)).collect() },
            Kind::Display => Command::Display { vars: need(0)? },
        })
    }

    fn in_boxes(&self, v: usize) -> bool {
        self.boxes.iter().any(|b| b.vars.contains(&v))
    }

    fn move_into(&mut self, bi: usize, v: usize) {
        let bx = &mut self.boxes[bi];
        if bx.max == 1 {
            bx.vars = vec![v];
        } else if bx.max == 0 || bx.vars.len() < bx.max {
            bx.vars.push(v);
        }
        self.src_sel = None;
    }

    pub fn ui(&mut self, ctx: &egui::Context, ds: &Dataset) -> Action {
        let mut action = Action::None;
        let mut open = true;
        let title = self.kind.title();
        egui::Window::new(title)
            .id(egui::Id::new("analysis_dialog"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .pivot(Align2::CENTER_CENTER)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                // drop variables that no longer exist
                for bx in &mut self.boxes {
                    bx.vars.retain(|&v| v < ds.vars.len());
                }
                let box_heights: Vec<f32> = self.boxes.iter().map(|b| if b.max == 1 { 24.0 } else { if self.boxes.len() > 1 { 96.0 } else { 220.0 } }).collect();
                let right_h: f32 = box_heights.iter().map(|h| h + 26.0).sum::<f32>();
                let list_h = right_h.max(200.0) - 8.0;
                ui.horizontal_top(|ui| {
                    // source list
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Variables:").size(12.5));
                        list_frame(ui, 250.0, list_h, |ui| {
                            for vi in 0..ds.vars.len() {
                                if self.in_boxes(vi) && self.kind != Kind::Display {
                                    continue;
                                }
                                if self.kind == Kind::Display && self.in_boxes(vi) {
                                    continue;
                                }
                                let r = var_item(ui, ds, vi, self.src_sel == Some(vi));
                                if r.clicked() {
                                    self.src_sel = Some(vi);
                                    self.box_sel = None;
                                }
                                if r.double_clicked() {
                                    if let Some(bi) = self.boxes.iter().position(|b| b.max == 0 || b.vars.len() < b.max) {
                                        self.move_into(bi, vi);
                                    }
                                }
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        for bi in 0..self.boxes.len() {
                            ui.add_space(2.0);
                            ui.label(RichText::new(self.boxes[bi].label).size(12.5));
                            ui.horizontal_top(|ui| {
                                let back = self.box_sel.map(|(b, _)| b == bi).unwrap_or(false);
                                let can = back || self.src_sel.is_some();
                                if arrow_button(ui, back, can).clicked() {
                                    if back {
                                        if let Some((_, pos)) = self.box_sel {
                                            if pos < self.boxes[bi].vars.len() {
                                                self.boxes[bi].vars.remove(pos);
                                            }
                                        }
                                        self.box_sel = None;
                                    } else if let Some(s) = self.src_sel {
                                        self.move_into(bi, s);
                                    }
                                }
                                list_frame(ui, 250.0, box_heights[bi], |ui| {
                                    let vars = self.boxes[bi].vars.clone();
                                    for (pos, vi) in vars.into_iter().enumerate() {
                                        let r = var_item(ui, ds, vi, self.box_sel == Some((bi, pos)));
                                        if r.clicked() {
                                            self.box_sel = Some((bi, pos));
                                            self.src_sel = None;
                                        }
                                        if r.double_clicked() {
                                            self.boxes[bi].vars.remove(pos);
                                            self.box_sel = None;
                                            break;
                                        }
                                    }
                                });
                            });
                        }
                    });
                });
                if !self.texts.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for (label, val) in &mut self.texts {
                            ui.label(*label);
                            ui.add(egui::TextEdit::singleline(val).desired_width(70.0));
                            ui.add_space(10.0);
                        }
                    });
                }
                if !self.choices.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(self.choice_label);
                        for (i, c) in self.choices.iter().enumerate() {
                            ui.radio_value(&mut self.choice, i, *c);
                        }
                    });
                }
                if !self.flags.is_empty() {
                    ui.add_space(6.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::Grid::new("dlg_flags").num_columns(3).spacing([16.0, 4.0]).show(ui, |ui| {
                            for (i, (name, val)) in self.flags.iter_mut().enumerate() {
                                ui.checkbox(val, *name);
                                if i % 3 == 2 {
                                    ui.end_row();
                                }
                            }
                        });
                    });
                }
                if let Some(e) = &self.error {
                    ui.add_space(4.0);
                    ui.colored_label(Color32::from_rgb(190, 40, 40), e);
                }
                if self.show_help {
                    ui.add_space(4.0);
                    egui::Frame::default().fill(Color32::from_rgb(255, 250, 230)).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(self.kind.help());
                    });
                }
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    let w = 74.0;
                    let total = w * 5.0 + 4.0 * 8.0;
                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                    if ui.add_sized([w, 26.0], egui::Button::new(RichText::new("OK").strong())).clicked() {
                        match self.to_command(ds) {
                            Ok(c) => action = Action::Ok(c),
                            Err(e) => self.error = Some(e),
                        }
                    }
                    if ui.add_sized([w, 26.0], egui::Button::new("Paste")).on_hover_text("Write the command into the syntax editor instead of running it").clicked() {
                        match self.to_command(ds) {
                            Ok(c) => action = Action::Paste(c),
                            Err(e) => self.error = Some(e),
                        }
                    }
                    if ui.add_sized([w, 26.0], egui::Button::new("Reset")).clicked() {
                        *self = Dialog::new(self.kind);
                    }
                    if ui.add_sized([w, 26.0], egui::Button::new("Cancel")).clicked() {
                        action = Action::Cancel;
                    }
                    if ui.add_sized([w, 26.0], egui::Button::new("Help")).clicked() {
                        self.show_help = !self.show_help;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    action = Action::Cancel;
                }
            });
        if !open {
            action = Action::Cancel;
        }
        action
    }
}

pub fn list_frame(ui: &mut egui::Ui, w: f32, h: f32, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::default().fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_gray(170))).inner_margin(egui::Margin::same(2)).show(ui, |ui| {
        ui.set_width(w);
        ui.set_height(h);
        egui::ScrollArea::vertical().id_salt(ui.next_auto_id()).auto_shrink([false, false]).min_scrolled_height(h).max_height(h).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui);
        });
    });
}

/// Small picture of a variable's measurement level.
pub fn measure_icon(p: &egui::Painter, r: Rect, numeric: bool, m: Measure) {
    let c = r.center();
    if !numeric {
        p.rect_filled(Rect::from_center_size(c, Vec2::new(12.0, 10.0)), 2.0, Color32::from_rgb(230, 120, 60));
        p.text(c, Align2::CENTER_CENTER, "a", egui::FontId::proportional(10.0), Color32::WHITE);
        return;
    }
    match m {
        Measure::Scale => {
            let rr = Rect::from_center_size(c, Vec2::new(13.0, 6.0));
            p.rect_filled(rr, 1.0, Color32::from_rgb(236, 196, 64));
            p.rect_stroke(rr, 1.0, Stroke::new(0.8, Color32::from_rgb(150, 110, 20)), StrokeKind::Inside);
            for k in 1..5 {
                let x = rr.min.x + k as f32 * 2.6;
                p.line_segment([Pos2::new(x, rr.min.y), Pos2::new(x, rr.min.y + if k % 2 == 0 { 3.5 } else { 2.0 })], Stroke::new(0.8, Color32::from_rgb(120, 90, 20)));
            }
        }
        Measure::Ordinal => {
            for k in 0..3 {
                let h = 4.0 + k as f32 * 3.0;
                let x = c.x - 5.0 + k as f32 * 4.0;
                p.rect_filled(Rect::from_min_max(Pos2::new(x, c.y + 5.0 - h), Pos2::new(x + 3.0, c.y + 5.0)), 0.0, Color32::from_rgb(70, 130, 200));
            }
        }
        Measure::Nominal => {
            let cols = [Color32::from_rgb(220, 70, 70), Color32::from_rgb(70, 160, 90), Color32::from_rgb(70, 110, 200)];
            for (k, col) in cols.iter().enumerate() {
                p.circle_filled(Pos2::new(c.x - 4.0 + k as f32 * 4.0, c.y + if k == 1 { -2.0 } else { 2.0 }), 2.3, *col);
            }
        }
    }
}

pub fn var_item(ui: &mut egui::Ui, ds: &Dataset, vi: usize, selected: bool) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::click());
    let v = &ds.vars[vi];
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 0.0, Color32::from_rgb(51, 122, 204));
    } else if resp.hovered() {
        p.rect_filled(rect, 0.0, Color32::from_rgb(230, 240, 252));
    }
    measure_icon(p, Rect::from_min_size(rect.min + Vec2::new(3.0, 3.0), Vec2::new(16.0, 14.0)), v.is_numeric(), v.measure);
    let col = if selected { Color32::WHITE } else { Color32::from_gray(25) };
    let text = v.display_name();
    let painter = ui.painter_at(rect);
    painter.text(Pos2::new(rect.min.x + 23.0, rect.center().y), Align2::LEFT_CENTER, text, egui::FontId::proportional(12.5), col);
    resp.on_hover_text(format!("{} ({}, {})", v.display_name(), v.type_name(), v.measure.name()))
}

pub fn arrow_button(ui: &mut egui::Ui, back: bool, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 24.0), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    let fill = if !enabled {
        Color32::from_rgb(160, 186, 222)
    } else if resp.hovered() {
        Color32::from_rgb(70, 140, 225)
    } else {
        Color32::from_rgb(46, 116, 204)
    };
    p.rect_filled(rect, 3.0, fill);
    p.rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::from_rgb(30, 80, 150)), StrokeKind::Inside);
    let c = rect.center();
    let col = Color32::WHITE;
    let pts = if back {
        vec![Pos2::new(c.x - 6.0, c.y), Pos2::new(c.x + 5.0, c.y - 6.0), Pos2::new(c.x + 5.0, c.y + 6.0)]
    } else {
        vec![Pos2::new(c.x + 6.0, c.y), Pos2::new(c.x - 5.0, c.y + 6.0), Pos2::new(c.x - 5.0, c.y - 6.0)]
    };
    p.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
    resp
}

// ---------------------------------------------------------------- Compute Variable

#[derive(Clone, Default)]
pub struct ComputeDlg {
    pub target: String,
    pub expr: String,
    pub src_sel: Option<usize>,
    pub func_sel: Option<usize>,
    pub error: Option<String>,
}

pub fn compute_ui(d: &mut ComputeDlg, ctx: &egui::Context, ds: &Dataset) -> Action {
    let mut action = Action::None;
    let mut open = true;
    egui::Window::new("Compute Variable").id(egui::Id::new("compute_dialog")).collapsible(false).resizable(false).open(&mut open).pivot(Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Target Variable:");
                ui.add(egui::TextEdit::singleline(&mut d.target).desired_width(180.0).hint_text("new or existing name"));
            });
            ui.label(RichText::new("=").size(20.0));
            ui.vertical(|ui| {
                ui.label("Numeric Expression:");
                ui.add(egui::TextEdit::multiline(&mut d.expr).desired_width(400.0).desired_rows(3).font(egui::TextStyle::Monospace));
            });
        });
        ui.add_space(6.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label("Variables (click to insert):");
                list_frame(ui, 200.0, 200.0, |ui| {
                    for vi in 0..ds.vars.len() {
                        let r = var_item(ui, ds, vi, d.src_sel == Some(vi));
                        if r.clicked() {
                            d.src_sel = Some(vi);
                            if !d.expr.is_empty() && !d.expr.ends_with(' ') && !d.expr.ends_with('(') {
                                d.expr.push(' ');
                            }
                            d.expr.push_str(&ds.vars[vi].name);
                        }
                    }
                });
            });
            ui.vertical(|ui| {
                ui.label("Keypad:");
                let keys = [["+", "<", ">", "7", "8", "9"], ["-", "<=", ">=", "4", "5", "6"], ["*", "=", "~=", "1", "2", "3"], ["/", "&", "|", "0", ".", "( )"], ["**", "~", "", "", "", "Delete"]];
                egui::Grid::new("keypad").spacing([3.0, 3.0]).show(ui, |ui| {
                    for row in keys {
                        for k in row {
                            if k.is_empty() {
                                ui.label("");
                                continue;
                            }
                            if ui.add_sized([if k == "Delete" { 52.0 } else { 34.0 }, 26.0], egui::Button::new(k)).clicked() {
                                match k {
                                    "Delete" => {
                                        d.expr.pop();
                                    }
                                    "( )" => d.expr.push_str("()"),
                                    _ => {
                                        let digit = k.chars().all(|c| c.is_ascii_digit() || c == '.');
                                        if digit {
                                            d.expr.push_str(k);
                                        } else {
                                            d.expr.push_str(&format!(" {} ", k));
                                        }
                                    }
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
            });
            ui.vertical(|ui| {
                ui.label("Functions (double-click to insert):");
                list_frame(ui, 230.0, 200.0, |ui| {
                    for (i, (f, desc)) in crate::expr::FUNCS.iter().enumerate() {
                        let r = ui.add_sized([224.0, 18.0], egui::SelectableLabel::new(d.func_sel == Some(i), *f)).on_hover_text(*desc);
                        if r.clicked() {
                            d.func_sel = Some(i);
                        }
                        if r.double_clicked() {
                            if *f == "$CASENUM" {
                                d.expr.push_str("$CASENUM");
                            } else {
                                d.expr.push_str(&format!("{}(", f));
                            }
                        }
                    }
                });
                if let Some(i) = d.func_sel {
                    ui.label(RichText::new(crate::expr::FUNCS[i].1).small());
                }
            });
        });
        if let Some(e) = &d.error {
            ui.colored_label(Color32::from_rgb(190, 40, 40), e);
        }
        ui.separator();
        ui.horizontal(|ui| {
            let make = |d: &mut ComputeDlg| -> Option<Command> {
                let t = d.target.trim();
                if t.is_empty() {
                    d.error = Some("Type a target variable name".into());
                    return None;
                }
                if let Err(e) = crate::expr::parse(&d.expr, ds) {
                    d.error = Some(e);
                    return None;
                }
                Some(Command::Compute { target: crate::data::valid_name(t), expr: d.expr.trim().to_string() })
            };
            if ui.add_sized([74.0, 26.0], egui::Button::new(RichText::new("OK").strong())).clicked() {
                if let Some(c) = make(d) {
                    action = Action::Ok(c);
                }
            }
            if ui.add_sized([74.0, 26.0], egui::Button::new("Paste")).clicked() {
                if let Some(c) = make(d) {
                    action = Action::Paste(c);
                }
            }
            if ui.add_sized([74.0, 26.0], egui::Button::new("Reset")).clicked() {
                *d = ComputeDlg::default();
            }
            if ui.add_sized([74.0, 26.0], egui::Button::new("Cancel")).clicked() {
                action = Action::Cancel;
            }
        });
    });
    if !open {
        action = Action::Cancel;
    }
    action
}

// ---------------------------------------------------------------- Select Cases

#[derive(Clone, Default)]
pub struct SelectDlg {
    pub use_if: bool,
    pub expr: String,
    pub error: Option<String>,
}

pub fn select_ui(d: &mut SelectDlg, ctx: &egui::Context, ds: &Dataset) -> Action {
    let mut action = Action::None;
    let mut open = true;
    egui::Window::new("Select Cases").id(egui::Id::new("select_dialog")).collapsible(false).resizable(false).open(&mut open).pivot(Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.label("Variables (click to insert):");
                list_frame(ui, 210.0, 220.0, |ui| {
                    for vi in 0..ds.vars.len() {
                        if var_item(ui, ds, vi, false).clicked() {
                            if !d.expr.is_empty() && !d.expr.ends_with(' ') {
                                d.expr.push(' ');
                            }
                            d.expr.push_str(&ds.vars[vi].name);
                            d.use_if = true;
                        }
                    }
                });
            });
            ui.vertical(|ui| {
                ui.label(RichText::new("Select").strong());
                ui.radio_value(&mut d.use_if, false, "All cases");
                ui.radio_value(&mut d.use_if, true, "If condition is satisfied:");
                ui.add_enabled(d.use_if, egui::TextEdit::multiline(&mut d.expr).desired_width(320.0).desired_rows(4).font(egui::TextStyle::Monospace).hint_text("for example  age >= 30 & gender = 2"));
                ui.add_space(6.0);
                ui.label(RichText::new("Unselected cases are filtered (kept, but left out of analyses).").small());
            });
        });
        if let Some(e) = &d.error {
            ui.colored_label(Color32::from_rgb(190, 40, 40), e);
        }
        ui.separator();
        ui.horizontal(|ui| {
            let make = |d: &mut SelectDlg| -> Option<Command> {
                if !d.use_if {
                    return Some(Command::Filter { expr: None });
                }
                match crate::expr::parse(&d.expr, ds) {
                    Ok(_) => Some(Command::Filter { expr: Some(d.expr.trim().to_string()) }),
                    Err(e) => {
                        d.error = Some(e);
                        None
                    }
                }
            };
            if ui.add_sized([74.0, 26.0], egui::Button::new(RichText::new("OK").strong())).clicked() {
                if let Some(c) = make(d) {
                    action = Action::Ok(c);
                }
            }
            if ui.add_sized([74.0, 26.0], egui::Button::new("Paste")).clicked() {
                if let Some(c) = make(d) {
                    action = Action::Paste(c);
                }
            }
            if ui.add_sized([74.0, 26.0], egui::Button::new("Reset")).clicked() {
                *d = SelectDlg::default();
            }
            if ui.add_sized([74.0, 26.0], egui::Button::new("Cancel")).clicked() {
                action = Action::Cancel;
            }
        });
    });
    if !open {
        action = Action::Cancel;
    }
    action
}
