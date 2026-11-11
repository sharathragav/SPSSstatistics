//! The views: data grid, variable view, output viewer, syntax editor, and small dialogs.

use crate::data::{Align, Measure, Role};
use crate::output::{self, Part};
use crate::{App, Edit, FindDlg, MissingDlg, SelMode, ValueLabelsDlg, View, APP_NAME};
use eframe::egui::{self, Align2, Color32, FontId, Id, Key, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

pub const ROW_H: f32 = 22.0;
pub const HDR_H: f32 = 24.0;
pub const RH_W: f32 = 56.0;
pub const SB: f32 = 14.0;

const GRID: Color32 = Color32::from_rgb(218, 222, 228);
const HEAD: Color32 = Color32::from_rgb(240, 242, 245);
const HEAD_SEL: Color32 = Color32::from_rgb(214, 228, 246);
const CELL_SEL: Color32 = Color32::from_rgb(255, 244, 196);
const LINE_SEL: Color32 = Color32::from_rgb(226, 236, 250);
const ACCENT: Color32 = Color32::from_rgb(40, 100, 190);

enum Hit {
    Corner,
    Col(usize),
    Row(usize),
    Cell(usize, usize),
}

impl App {
    pub fn col_w(&self, c: usize) -> f32 {
        if c < self.ds.vars.len() {
            (self.ds.vars[c].columns as f32 * 8.6 + 18.0).clamp(46.0, 420.0)
        } else {
            74.0
        }
    }

    pub fn commit_edit(&mut self) {
        if let Some(e) = self.editing.take() {
            let cur = if e.c < self.ds.vars.len() && e.r < self.ds.ncases { self.ds.vars[e.c].edit_text(e.r) } else { String::new() };
            if e.text == cur {
                return;
            }
            let backup = self.ds.clone();
            if self.ds.set_cell(e.r, e.c, &e.text) {
                self.undo.push(backup);
                if self.undo.len() > 40 {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.dirty = true;
                self.status.clear();
            } else {
                self.ds = backup;
                self.status = format!("'{}' isn't a number, so it can't go in the numeric variable {}. Change its type in Variable View to store text.", e.text.trim(), self.ds.vars.get(e.c).map(|v| v.name.as_str()).unwrap_or(""));
            }
            self.bar_for = (usize::MAX, usize::MAX, usize::MAX);
        }
    }

    fn start_edit(&mut self, initial: Option<String>) {
        let (r, c) = self.sel;
        let text = match initial {
            Some(t) => t,
            None => {
                if c < self.ds.vars.len() && r < self.ds.ncases {
                    self.ds.vars[c].edit_text(r)
                } else {
                    String::new()
                }
            }
        };
        self.sel_mode = SelMode::Cell;
        self.editing = Some(Edit { r, c, text, focus: true });
    }

    fn vis_rows(&self, h: f32) -> usize {
        (((h - HDR_H - SB) / ROW_H).floor() as usize).max(1)
    }

    fn ensure_visible(&mut self, vis_rows: usize, grid_w: f32) {
        let (r, c) = self.sel;
        if r < self.top_row {
            self.top_row = r;
        }
        if r >= self.top_row + vis_rows {
            self.top_row = r + 1 - vis_rows;
        }
        if c < self.left_col {
            self.left_col = c;
        }
        loop {
            let mut x = RH_W;
            let mut last = self.left_col;
            let mut k = self.left_col;
            while x < grid_w {
                x += self.col_w(k);
                if x <= grid_w {
                    last = k;
                }
                k += 1;
            }
            if c > last && self.left_col < c {
                self.left_col += 1;
            } else {
                break;
            }
        }
    }

    pub fn selection_text(&self) -> String {
        let (r, c) = self.sel;
        match self.sel_mode {
            SelMode::Cell => {
                if c < self.ds.vars.len() && r < self.ds.ncases {
                    let t = self.ds.vars[c].cell_text(r, self.labels_on);
                    if t == "." {
                        String::new()
                    } else {
                        t
                    }
                } else {
                    String::new()
                }
            }
            SelMode::Col => {
                if c < self.ds.vars.len() {
                    let v = &self.ds.vars[c];
                    let mut s = vec![v.name.clone()];
                    for i in 0..self.ds.ncases {
                        s.push(v.edit_text(i));
                    }
                    s.join("\n")
                } else {
                    String::new()
                }
            }
            SelMode::Row => {
                if r < self.ds.ncases {
                    self.ds.vars.iter().map(|v| v.edit_text(r)).collect::<Vec<_>>().join("\t")
                } else {
                    String::new()
                }
            }
        }
    }

    pub fn clear_selection(&mut self) {
        let (r, c) = self.sel;
        match self.sel_mode {
            SelMode::Cell => {
                if c < self.ds.vars.len() && r < self.ds.ncases {
                    self.push_undo();
                    self.ds.vars[c].set_text(r, "");
                }
            }
            SelMode::Col => {
                if c < self.ds.vars.len() {
                    self.push_undo();
                    self.ds.delete_var(c);
                    self.status = "Variable deleted.".into();
                }
            }
            SelMode::Row => {
                if r < self.ds.ncases {
                    self.push_undo();
                    self.ds.delete_case(r);
                    self.status = "Case deleted.".into();
                }
            }
        }
        self.bar_for = (usize::MAX, usize::MAX, usize::MAX);
    }

    pub fn insert_variable(&mut self) {
        self.push_undo();
        let at = if self.view == View::Data { self.sel.1.min(self.ds.vars.len()) } else { self.sel.0.min(self.ds.vars.len()) };
        self.ds.insert_var(at);
        self.status = format!("Inserted variable {}.", self.ds.vars[at].name);
    }

    pub fn insert_case(&mut self) {
        if self.ds.vars.is_empty() {
            self.ds.ensure_vars(0);
        }
        self.push_undo();
        let at = self.sel.0.min(self.ds.ncases);
        self.ds.insert_case(at);
        self.status = format!("Inserted a case at row {}.", at + 1);
    }

    fn paste_block(&mut self, text: &str) {
        let rows: Vec<Vec<String>> = text.trim_end_matches(['\n', '\r']).lines().map(|l| l.split('\t').map(|s| s.to_string()).collect()).collect();
        if rows.is_empty() {
            return;
        }
        let backup = self.ds.clone();
        let (r0, c0) = self.sel;
        let mut bad = 0;
        for (i, row) in rows.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if !self.ds.set_cell(r0 + i, c0 + j, cell) {
                    bad += 1;
                }
            }
        }
        self.undo.push(backup);
        self.redo.clear();
        self.dirty = true;
        self.status = if bad > 0 { format!("Pasted; {} values didn't fit numeric variables and were skipped.", bad) } else { format!("Pasted {} rows.", rows.len()) };
        self.bar_for = (usize::MAX, usize::MAX, usize::MAX);
    }

    pub fn grid_keys(&mut self, ctx: &egui::Context) {
        if self.editing.is_some() || ctx.wants_keyboard_input() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let screen = ctx.screen_rect();
        let vis = self.vis_rows(screen.height() - 120.0);
        let mut moved = false;
        for ev in events {
            match ev {
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    let (r, c) = self.sel;
                    match key {
                        Key::ArrowDown | Key::Enter => {
                            self.sel = (r + 1, c);
                            moved = true;
                        }
                        Key::ArrowUp => {
                            self.sel = (r.saturating_sub(1), c);
                            moved = true;
                        }
                        Key::ArrowRight => {
                            self.sel = (r, c + 1);
                            moved = true;
                        }
                        Key::ArrowLeft => {
                            self.sel = (r, c.saturating_sub(1));
                            moved = true;
                        }
                        Key::Tab => {
                            self.sel = if modifiers.shift { (r, c.saturating_sub(1)) } else { (r, c + 1) };
                            moved = true;
                        }
                        Key::PageDown => {
                            self.sel = (r + vis, c);
                            moved = true;
                        }
                        Key::PageUp => {
                            self.sel = (r.saturating_sub(vis), c);
                            moved = true;
                        }
                        Key::Home => {
                            self.sel = if modifiers.command { (0, 0) } else { (r, 0) };
                            moved = true;
                        }
                        Key::End => {
                            self.sel = if modifiers.command { (self.ds.ncases.saturating_sub(1), self.ds.vars.len().saturating_sub(1)) } else { (r, self.ds.vars.len().saturating_sub(1)) };
                            moved = true;
                        }
                        Key::Delete | Key::Backspace => self.clear_selection(),
                        Key::F2 => self.start_edit(None),
                        _ => {}
                    }
                    if moved {
                        self.sel_mode = SelMode::Cell;
                    }
                }
                egui::Event::Text(t) => {
                    if !t.chars().all(|c| c.is_control()) && self.sel_mode == SelMode::Cell {
                        self.start_edit(Some(t));
                    }
                }
                egui::Event::Copy => {
                    let t = self.selection_text();
                    ctx.copy_text(t);
                }
                egui::Event::Cut => {
                    let t = self.selection_text();
                    ctx.copy_text(t);
                    self.clear_selection();
                }
                egui::Event::Paste(s) => self.paste_block(&s),
                _ => {}
            }
        }
        if moved {
            let w = screen.width() - SB;
            self.ensure_visible(vis, w);
        }
    }

    pub fn cell_bar(&mut self, ui: &mut egui::Ui) {
        let (r, c) = self.sel;
        let key = (r, c, self.ds.vars.len() * 1_000_003 + self.ds.ncases);
        let bar_id = Id::new("cell_bar_edit");
        let bar_focused = ui.memory(|m| m.has_focus(bar_id));
        if self.bar_for != key && !bar_focused {
            self.bar_text = if c < self.ds.vars.len() && r < self.ds.ncases { self.ds.vars[c].edit_text(r) } else { String::new() };
            self.bar_for = key;
        }
        ui.horizontal(|ui| {
            let name = if c < self.ds.vars.len() { self.ds.vars[c].name.clone() } else { String::new() };
            let label = if self.sel_mode == SelMode::Cell { format!("{} : {}", r + 1, name) } else { String::new() };
            egui::Frame::default().fill(Color32::from_rgb(246, 247, 249)).stroke(Stroke::new(1.0, Color32::from_gray(205))).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
                ui.set_width(150.0);
                ui.label(RichText::new(label).size(12.5));
            });
            let shown = match &self.editing {
                Some(e) => e.text.clone(),
                None => self.bar_text.clone(),
            };
            let mut text = shown.clone();
            let w = (ui.available_width() - 190.0).max(100.0);
            let resp = ui.add(egui::TextEdit::singleline(&mut text).id(bar_id).desired_width(w).font(egui::TextStyle::Body));
            if self.editing.is_none() {
                self.bar_text = text;
            } else if text != shown {
                if let Some(e) = &mut self.editing {
                    e.text = text;
                }
            }
            if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) && self.editing.is_none() {
                let t = self.bar_text.clone();
                self.editing = Some(Edit { r, c, text: t, focus: false });
                self.commit_edit();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let n = self.ds.vars.len();
                ui.label(RichText::new(format!("Visible: {} of {} Variables", n, n)).size(12.0).color(Color32::from_gray(80)));
            });
        });
    }

    fn hit(&self, pos: Pos2, grid: Rect) -> Option<Hit> {
        if !grid.contains(pos) {
            return None;
        }
        let x = pos.x - grid.min.x;
        let y = pos.y - grid.min.y;
        let col = if x < RH_W {
            None
        } else {
            let mut cx = RH_W;
            let mut c = self.left_col;
            loop {
                let w = self.col_w(c);
                if x < cx + w {
                    break Some(c);
                }
                cx += w;
                c += 1;
                if c > self.left_col + 400 {
                    break None;
                }
            }
        };
        let row = if y < HDR_H { None } else { Some(self.top_row + ((y - HDR_H) / ROW_H) as usize) };
        Some(match (row, col) {
            (None, None) => Hit::Corner,
            (None, Some(c)) => Hit::Col(c),
            (Some(r), None) => Hit::Row(r),
            (Some(r), Some(c)) => Hit::Cell(r, c),
        })
    }

    pub fn data_view(&mut self, ui: &mut egui::Ui) {
        let full = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(full, Sense::click_and_drag());
        let grid = Rect::from_min_max(full.min, Pos2::new(full.max.x - SB, full.max.y - SB));
        let vis_rows = self.vis_rows(full.height());
        let total_rows = (self.ds.ncases + vis_rows).max(1000);
        let total_cols = (self.ds.vars.len() + 40).max(60);
        // wheel scrolling
        if resp.hovered() && !self.modal_open() {
            let d = ui.input(|i| i.smooth_scroll_delta);
            self.scroll_acc += d;
            while self.scroll_acc.y <= -ROW_H {
                self.top_row += 1;
                self.scroll_acc.y += ROW_H;
            }
            while self.scroll_acc.y >= ROW_H {
                self.top_row = self.top_row.saturating_sub(1);
                self.scroll_acc.y -= ROW_H;
            }
            while self.scroll_acc.x <= -60.0 {
                self.left_col += 1;
                self.scroll_acc.x += 60.0;
            }
            while self.scroll_acc.x >= 60.0 {
                self.left_col = self.left_col.saturating_sub(1);
                self.scroll_acc.x -= 60.0;
            }
        }
        self.top_row = self.top_row.min(total_rows.saturating_sub(vis_rows));
        self.left_col = self.left_col.min(total_cols.saturating_sub(1));
        let p = ui.painter_at(full);
        p.rect_filled(full, 0.0, Color32::WHITE);
        // visible columns
        let mut cols: Vec<(usize, f32, f32)> = vec![];
        let mut x = grid.min.x + RH_W;
        let mut c = self.left_col;
        while x < grid.max.x && c < total_cols {
            let w = self.col_w(c);
            cols.push((c, x, w));
            x += w;
            c += 1;
        }
        let font = FontId::proportional(13.0);
        let nvars = self.ds.vars.len();
        let ncases = self.ds.ncases;
        let (sr, sc) = self.sel;
        // body
        for i in 0..=vis_rows {
            let r = self.top_row + i;
            let y = grid.min.y + HDR_H + i as f32 * ROW_H;
            if y > grid.max.y {
                break;
            }
            let row_sel = self.sel_mode == SelMode::Row && r == sr;
            for &(c, x, w) in &cols {
                let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, ROW_H));
                let col_sel = self.sel_mode == SelMode::Col && c == sc;
                if row_sel || col_sel {
                    p.rect_filled(rect, 0.0, LINE_SEL);
                } else if self.sel_mode == SelMode::Cell && r == sr && c == sc {
                    p.rect_filled(rect, 0.0, CELL_SEL);
                }
                if c < nvars && r < ncases {
                    let v = &self.ds.vars[c];
                    let t = v.cell_text(r, self.labels_on);
                    if !t.is_empty() {
                        let (pos, al) = match (v.align, self.labels_on && !v.value_labels.is_empty()) {
                            (_, true) | (Align::Left, _) => (Pos2::new(rect.min.x + 5.0, rect.center().y), Align2::LEFT_CENTER),
                            (Align::Center, _) => (rect.center(), Align2::CENTER_CENTER),
                            (Align::Right, _) => (Pos2::new(rect.max.x - 5.0, rect.center().y), Align2::RIGHT_CENTER),
                        };
                        let cp = p.with_clip_rect(rect.shrink(1.0).intersect(grid));
                        cp.text(pos, al, t, font.clone(), Color32::from_gray(20));
                    }
                }
                if self.grid_lines {
                    p.line_segment([Pos2::new(rect.max.x, y), Pos2::new(rect.max.x, y + ROW_H)], Stroke::new(1.0, GRID));
                }
            }
            if self.grid_lines {
                p.line_segment([Pos2::new(grid.min.x + RH_W, y + ROW_H), Pos2::new(grid.max.x, y + ROW_H)], Stroke::new(1.0, GRID));
            }
            // row header
            let rh = Rect::from_min_size(Pos2::new(grid.min.x, y), Vec2::new(RH_W, ROW_H));
            let hl = r == sr && (self.sel_mode != SelMode::Col);
            p.rect_filled(rh, 0.0, if hl { HEAD_SEL } else { HEAD });
            p.line_segment([Pos2::new(rh.min.x, rh.max.y), Pos2::new(rh.max.x, rh.max.y)], Stroke::new(1.0, Color32::from_gray(205)));
            let filtered = r < ncases && !self.ds.included(r);
            p.text(Pos2::new(rh.max.x - 8.0, rh.center().y), Align2::RIGHT_CENTER, format!("{}", r + 1), FontId::proportional(12.0), if r < ncases { Color32::from_gray(40) } else { Color32::from_gray(150) });
            if filtered {
                p.line_segment([Pos2::new(rh.min.x + 10.0, rh.max.y - 4.0), Pos2::new(rh.max.x - 6.0, rh.min.y + 4.0)], Stroke::new(1.5, Color32::from_rgb(190, 60, 40)));
            }
        }
        // header row
        let corner = Rect::from_min_size(grid.min, Vec2::new(RH_W, HDR_H));
        p.rect_filled(Rect::from_min_max(grid.min, Pos2::new(grid.max.x, grid.min.y + HDR_H)), 0.0, HEAD);
        for &(c, x, w) in &cols {
            let rect = Rect::from_min_size(Pos2::new(x, grid.min.y), Vec2::new(w, HDR_H));
            let hl = c == sc && self.sel_mode != SelMode::Row;
            if hl {
                p.rect_filled(rect, 0.0, HEAD_SEL);
            }
            let cp = p.with_clip_rect(rect.shrink(1.0));
            if c < nvars {
                let v = &self.ds.vars[c];
                crate::dialogs::measure_icon(&cp, Rect::from_min_size(Pos2::new(rect.min.x + 3.0, rect.min.y + 5.0), Vec2::new(16.0, 14.0)), v.is_numeric(), v.measure);
                cp.text(Pos2::new(rect.min.x + 22.0, rect.center().y), Align2::LEFT_CENTER, &v.name, FontId::proportional(13.0), Color32::from_gray(15));
            } else {
                cp.text(rect.center(), Align2::CENTER_CENTER, "var", FontId::proportional(13.0), Color32::from_gray(150));
            }
            p.line_segment([Pos2::new(rect.max.x, rect.min.y), Pos2::new(rect.max.x, rect.max.y)], Stroke::new(1.0, Color32::from_gray(205)));
        }
        p.rect_filled(corner, 0.0, HEAD);
        p.line_segment([Pos2::new(grid.min.x, grid.min.y + HDR_H), Pos2::new(grid.max.x, grid.min.y + HDR_H)], Stroke::new(1.0, Color32::from_gray(185)));
        p.line_segment([Pos2::new(grid.min.x + RH_W, grid.min.y), Pos2::new(grid.min.x + RH_W, grid.max.y)], Stroke::new(1.0, Color32::from_gray(185)));
        // selected cell outline
        if self.sel_mode == SelMode::Cell && sr >= self.top_row {
            if let Some(&(_, x, w)) = cols.iter().find(|(c, _, _)| *c == sc) {
                let y = grid.min.y + HDR_H + (sr - self.top_row) as f32 * ROW_H;
                if y < grid.max.y {
                    p.rect_stroke(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, ROW_H)), 0.0, Stroke::new(2.0, ACCENT), StrokeKind::Inside);
                }
            }
        }
        // empty-sheet hint
        if nvars == 0 && ncases == 0 && !self.welcome {
            let r = Rect::from_center_size(grid.center(), Vec2::new(440.0, 54.0));
            p.rect_filled(r, 6.0, Color32::from_rgba_unmultiplied(40, 70, 120, 220));
            p.text(r.center() - Vec2::new(0.0, 9.0), Align2::CENTER_CENTER, "Type values to start a dataset, or open a file.", FontId::proportional(14.0), Color32::WHITE);
            p.text(r.center() + Vec2::new(0.0, 11.0), Align2::CENTER_CENTER, "Tip: File > Open Sample Dataset has example data to explore.", FontId::proportional(12.0), Color32::from_rgb(210, 225, 245));
        }
        // mouse
        let modal = self.modal_open();
        if !modal && (resp.clicked() || resp.double_clicked() || resp.secondary_clicked() || resp.drag_started()) {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some(h) = self.hit(pos, grid) {
                    if self.editing.is_some() {
                        self.commit_edit();
                    }
                    match h {
                        Hit::Corner => {
                            self.sel = (0, 0);
                            self.sel_mode = SelMode::Cell;
                        }
                        Hit::Col(c) => {
                            self.sel = (self.sel.0, c);
                            self.sel_mode = SelMode::Col;
                            if resp.double_clicked() && c < nvars {
                                self.view = View::Variables;
                                self.sel = (c, 0);
                            }
                        }
                        Hit::Row(r) => {
                            self.sel = (r, self.sel.1);
                            self.sel_mode = SelMode::Row;
                        }
                        Hit::Cell(r, c) => {
                            self.sel = (r, c);
                            self.sel_mode = SelMode::Cell;
                            if resp.double_clicked() {
                                self.start_edit(None);
                            }
                        }
                    }
                }
            }
        }
        let sel_mode = self.sel_mode;
        let sel_c = self.sel.1;
        resp.context_menu(|ui| {
            match sel_mode {
                SelMode::Col => {
                    if ui.button("Insert Variable").clicked() {
                        self.insert_variable();
                        ui.close_menu();
                    }
                    if sel_c < nvars {
                        if ui.button("Clear (delete variable)").clicked() {
                            self.clear_selection();
                            ui.close_menu();
                        }
                        if ui.button("Copy").clicked() {
                            let t = self.selection_text();
                            ui.ctx().copy_text(t);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Sort Ascending").clicked() {
                            let n = self.ds.vars[sel_c].name.clone();
                            self.run(crate::syntax::Command::Sort { keys: vec![(n, true)] });
                            ui.close_menu();
                        }
                        if ui.button("Sort Descending").clicked() {
                            let n = self.ds.vars[sel_c].name.clone();
                            self.run(crate::syntax::Command::Sort { keys: vec![(n, false)] });
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Descriptive Statistics").clicked() {
                            let n = self.ds.vars[sel_c].name.clone();
                            if self.ds.vars[sel_c].is_numeric() {
                                self.run(crate::syntax::Command::Descriptives { vars: vec![n], stats: crate::output::sv(&["MEAN", "STDDEV", "MIN", "MAX"]), save: false });
                            } else {
                                self.run(crate::syntax::Command::Frequencies { vars: vec![n], table: true, stats: vec![], chart: String::new(), normal: false });
                            }
                            ui.close_menu();
                        }
                        if ui.button("Variable Information").clicked() {
                            self.view = View::Variables;
                            self.sel = (sel_c, 0);
                            ui.close_menu();
                        }
                    }
                }
                SelMode::Row => {
                    if ui.button("Insert Cases").clicked() {
                        self.insert_case();
                        ui.close_menu();
                    }
                    if ui.button("Clear (delete case)").clicked() {
                        self.clear_selection();
                        ui.close_menu();
                    }
                    if ui.button("Copy").clicked() {
                        let t = self.selection_text();
                        ui.ctx().copy_text(t);
                        ui.close_menu();
                    }
                }
                SelMode::Cell => {
                    if ui.button("Copy").clicked() {
                        let t = self.selection_text();
                        ui.ctx().copy_text(t);
                        ui.close_menu();
                    }
                    if ui.button("Clear").clicked() {
                        self.clear_selection();
                        ui.close_menu();
                    }
                    if ui.button("Edit Cell").clicked() {
                        self.start_edit(None);
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
                }
            }
        });
        // in-cell editor
        let mut finish: Option<(bool, i32, i32)> = None; // (commit, dr, dc)
        if let Some(e) = &mut self.editing {
            let vis = cols.iter().find(|(c, _, _)| *c == e.c).copied();
            if let (Some((_, x, w)), true) = (vis, e.r >= self.top_row && e.r < self.top_row + vis_rows + 1) {
                let y = grid.min.y + HDR_H + (e.r - self.top_row) as f32 * ROW_H;
                let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w.max(120.0), ROW_H));
                let id = Id::new("cell_editor");
                ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
                let te = egui::TextEdit::singleline(&mut e.text).id(id).frame(false).margin(egui::Margin::symmetric(4, 3)).font(egui::TextStyle::Body);
                let r = ui.put(rect, te);
                ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0, ACCENT), StrokeKind::Inside);
                if e.focus {
                    r.request_focus();
                    // put the cursor at the end
                    if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), id) {
                        let cc = egui::text::CCursor::new(e.text.chars().count());
                        st.cursor.set_char_range(Some(egui::text::CCursorRange::one(cc)));
                        st.store(ui.ctx(), id);
                    }
                    e.focus = false;
                } else if r.lost_focus() || !r.has_focus() {
                    let (enter, tab, shift, esc, up, down) = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Tab), i.modifiers.shift, i.key_pressed(Key::Escape), i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)));
                    finish = Some(if esc {
                        (false, 0, 0)
                    } else if enter || down {
                        (true, 1, 0)
                    } else if up {
                        (true, -1, 0)
                    } else if tab {
                        (true, 0, if shift { -1 } else { 1 })
                    } else {
                        (true, 0, 0)
                    });
                } else {
                    let (up, down) = ui.input(|i| (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)));
                    if down {
                        finish = Some((true, 1, 0));
                    } else if up {
                        finish = Some((true, -1, 0));
                    }
                }
            } else {
                finish = Some((true, 0, 0));
            }
        }
        if let Some((commit, dr, dc)) = finish {
            let (r, c) = self.editing.as_ref().map(|e| (e.r, e.c)).unwrap_or(self.sel);
            if commit {
                self.commit_edit();
            } else {
                self.editing = None;
            }
            self.sel = ((r as i64 + dr as i64).max(0) as usize, (c as i64 + dc as i64).max(0) as usize);
            self.sel_mode = SelMode::Cell;
            let w = grid.width();
            self.ensure_visible(vis_rows, w);
        }
        // scrollbars
        let vtrack = Rect::from_min_max(Pos2::new(grid.max.x, grid.min.y + HDR_H), Pos2::new(full.max.x, grid.max.y));
        let htrack = Rect::from_min_max(Pos2::new(grid.min.x + RH_W, grid.max.y), Pos2::new(grid.max.x, full.max.y));
        let p = ui.painter_at(full);
        p.rect_filled(vtrack, 0.0, Color32::from_rgb(240, 241, 243));
        p.rect_filled(htrack, 0.0, Color32::from_rgb(240, 241, 243));
        p.rect_filled(Rect::from_min_max(Pos2::new(grid.max.x, grid.max.y), full.max), 0.0, Color32::from_rgb(232, 233, 236));
        p.rect_filled(Rect::from_min_max(Pos2::new(grid.min.x, grid.max.y), Pos2::new(grid.min.x + RH_W, full.max.y)), 0.0, Color32::from_rgb(232, 233, 236));
        let vmax = total_rows.saturating_sub(vis_rows).max(1);
        let th = (vtrack.height() * vis_rows as f32 / total_rows as f32).clamp(24.0, vtrack.height());
        let ty = vtrack.min.y + (vtrack.height() - th) * (self.top_row as f32 / vmax as f32);
        let vthumb = Rect::from_min_size(Pos2::new(vtrack.min.x + 2.0, ty), Vec2::new(SB - 4.0, th));
        let vr = ui.interact(vtrack, Id::new("vscroll"), Sense::click_and_drag());
        p.rect_filled(vthumb, 4.0, if vr.hovered() || vr.dragged() { Color32::from_gray(150) } else { Color32::from_gray(185) });
        if vr.dragged() || vr.clicked() {
            if let Some(pos) = vr.interact_pointer_pos() {
                let f = ((pos.y - vtrack.min.y - th / 2.0) / (vtrack.height() - th).max(1.0)).clamp(0.0, 1.0);
                self.top_row = (f * vmax as f32).round() as usize;
            }
        }
        let cmax = total_cols.saturating_sub(1).max(1);
        let tw = (htrack.width() * (cols.len() as f32 / total_cols as f32)).clamp(30.0, htrack.width());
        let tx = htrack.min.x + (htrack.width() - tw) * (self.left_col as f32 / cmax as f32);
        let hthumb = Rect::from_min_size(Pos2::new(tx, htrack.min.y + 2.0), Vec2::new(tw, SB - 4.0));
        let hr = ui.interact(htrack, Id::new("hscroll"), Sense::click_and_drag());
        p.rect_filled(hthumb, 4.0, if hr.hovered() || hr.dragged() { Color32::from_gray(150) } else { Color32::from_gray(185) });
        if hr.dragged() || hr.clicked() {
            if let Some(pos) = hr.interact_pointer_pos() {
                let f = ((pos.x - htrack.min.x - tw / 2.0) / (htrack.width() - tw).max(1.0)).clamp(0.0, 1.0);
                self.left_col = (f * cmax as f32).round() as usize;
            }
        }
    }

    // ------------------------------------------------------------ variable view

    pub fn variable_view(&mut self, ui: &mut egui::Ui) {
        let heads: [(&str, f32); 11] = [
            ("Name", 120.0),
            ("Type", 92.0),
            ("Width", 60.0),
            ("Decimals", 70.0),
            ("Label", 230.0),
            ("Values", 150.0),
            ("Missing", 100.0),
            ("Columns", 70.0),
            ("Align", 82.0),
            ("Measure", 98.0),
            ("Role", 86.0),
        ];
        let rh = 24.0;
        let full = ui.available_rect_before_wrap();
        let p = ui.painter().clone();
        // header
        let hy = full.min.y;
        p.rect_filled(Rect::from_min_size(full.min, Vec2::new(full.width(), HDR_H)), 0.0, HEAD);
        let mut x = full.min.x + RH_W;
        for (h, w) in heads.iter() {
            let r = Rect::from_min_size(Pos2::new(x, hy), Vec2::new(*w, HDR_H));
            p.text(Pos2::new(r.min.x + 6.0, r.center().y), Align2::LEFT_CENTER, *h, FontId::proportional(13.0), Color32::from_gray(20));
            p.line_segment([Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.max.y)], Stroke::new(1.0, Color32::from_gray(205)));
            x += w;
        }
        p.line_segment([Pos2::new(full.min.x, hy + HDR_H), Pos2::new(full.max.x, hy + HDR_H)], Stroke::new(1.0, Color32::from_gray(185)));
        ui.allocate_space(Vec2::new(full.width(), HDR_H));
        let nvars = self.ds.vars.len();
        let nrows = nvars + 40;
        let mut action: Option<(u8, usize)> = None; // 0 insert, 1 delete, 2 values, 3 missing, 4 select, 5 duplicate
        egui::ScrollArea::vertical().id_salt("varview").auto_shrink([false, false]).show_rows(ui, rh, nrows, |ui, range| {
            for r in range {
                let (row_rect, _) = ui.allocate_exact_size(Vec2::new(full.width(), rh), Sense::hover());
                let p = ui.painter();
                let selected = self.sel.0 == r;
                let hdr = Rect::from_min_size(row_rect.min, Vec2::new(RH_W, rh));
                p.rect_filled(hdr, 0.0, if selected { HEAD_SEL } else { HEAD });
                p.text(Pos2::new(hdr.max.x - 8.0, hdr.center().y), Align2::RIGHT_CENTER, format!("{}", r + 1), FontId::proportional(12.0), if r < nvars { Color32::from_gray(40) } else { Color32::from_gray(150) });
                let hresp = ui.interact(hdr, Id::new(("vvrow", r)), Sense::click());
                if hresp.clicked() {
                    action = Some((4, r));
                }
                if r < nvars {
                    hresp.context_menu(|ui| {
                        if ui.button("Insert Variable").clicked() {
                            action = Some((0, r));
                            ui.close_menu();
                        }
                        if ui.button("Duplicate Variable").clicked() {
                            action = Some((5, r));
                            ui.close_menu();
                        }
                        if ui.button("Clear (delete variable)").clicked() {
                            action = Some((1, r));
                            ui.close_menu();
                        }
                    });
                }
                let mut x = row_rect.min.x + RH_W;
                let mut cells: Vec<Rect> = vec![];
                for (_, w) in heads.iter() {
                    cells.push(Rect::from_min_size(Pos2::new(x, row_rect.min.y), Vec2::new(*w, rh)));
                    x += w;
                }
                if selected && r < nvars {
                    p.rect_filled(Rect::from_min_max(cells[0].min, cells[10].max), 0.0, Color32::from_rgb(250, 252, 255));
                }
                for c in &cells {
                    p.line_segment([Pos2::new(c.max.x, c.min.y), Pos2::new(c.max.x, c.max.y)], Stroke::new(1.0, GRID));
                }
                p.line_segment([Pos2::new(row_rect.min.x, row_rect.max.y), Pos2::new(row_rect.max.x, row_rect.max.y)], Stroke::new(1.0, GRID));
                if r < nvars {
                    let inner = |c: Rect| c.shrink2(Vec2::new(2.0, 2.0));
                    // name
                    let nid = Id::new(("vname", r));
                    let resp = ui.put(inner(cells[0]), egui::TextEdit::singleline(&mut self.ds.vars[r].name).id(nid).frame(false).margin(egui::Margin::symmetric(4, 2)));
                    if resp.gained_focus() {
                        self.undo.push(self.ds.clone());
                        self.sel = (r, 0);
                    }
                    if resp.changed() {
                        self.dirty = true;
                    }
                    if resp.lost_focus() {
                        let n = self.ds.vars[r].name.clone();
                        let fixed = self.ds.unique_name(&n, Some(r));
                        self.ds.vars[r].name = fixed;
                    }
                    // type
                    let mut numeric = self.ds.vars[r].is_numeric();
                    let before = numeric;
                    ui.put(inner(cells[1]), |ui: &mut egui::Ui| {
                        egui::ComboBox::from_id_salt(("vtype", r))
                            .selected_text(if numeric { "Numeric" } else { "String" })
                            .width(cells[1].width() - 8.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut numeric, true, "Numeric");
                                ui.selectable_value(&mut numeric, false, "String");
                            })
                            .response
                    });
                    if numeric != before {
                        self.push_undo();
                        self.ds.vars[r].set_numeric(numeric);
                    }
                    let v = &mut self.ds.vars[r];
                    let w0 = v.width;
                    ui.put(inner(cells[2]), egui::DragValue::new(&mut v.width).range(1..=255));
                    if v.is_numeric() {
                        ui.put(inner(cells[3]), egui::DragValue::new(&mut v.decimals).range(0..=16));
                    } else {
                        ui.painter().text(Pos2::new(cells[3].min.x + 8.0, cells[3].center().y), Align2::LEFT_CENTER, "0", FontId::proportional(13.0), Color32::from_gray(140));
                    }
                    let lr = ui.put(inner(cells[4]), egui::TextEdit::singleline(&mut v.label).frame(false).margin(egui::Margin::symmetric(4, 2)).hint_text(""));
                    if lr.changed() || v.width != w0 {
                        self.dirty = true;
                    }
                    let v = &mut self.ds.vars[r];
                    if ui.put(inner(cells[5]), egui::Button::new(RichText::new(v.values_summary()).size(12.5)).frame(false)).on_hover_text("Edit value labels").clicked() {
                        action = Some((2, r));
                    }
                    if ui.put(inner(cells[6]), egui::Button::new(RichText::new(v.missing_summary()).size(12.5)).frame(false)).on_hover_text("Edit missing values").clicked() {
                        action = Some((3, r));
                    }
                    ui.put(inner(cells[7]), egui::DragValue::new(&mut v.columns).range(1..=255));
                    let mut al = v.align;
                    ui.put(inner(cells[8]), |ui: &mut egui::Ui| {
                        egui::ComboBox::from_id_salt(("valign", r))
                            .selected_text(al.name())
                            .width(cells[8].width() - 8.0)
                            .show_ui(ui, |ui| {
                                for a in Align::ALL {
                                    ui.selectable_value(&mut al, a, a.name());
                                }
                            })
                            .response
                    });
                    let mut me = v.measure;
                    ui.put(inner(cells[9]), |ui: &mut egui::Ui| {
                        egui::ComboBox::from_id_salt(("vmeasure", r))
                            .selected_text(me.name())
                            .width(cells[9].width() - 8.0)
                            .show_ui(ui, |ui| {
                                for m in Measure::ALL {
                                    ui.selectable_value(&mut me, m, m.name());
                                }
                            })
                            .response
                    });
                    let mut ro = v.role;
                    ui.put(inner(cells[10]), |ui: &mut egui::Ui| {
                        egui::ComboBox::from_id_salt(("vrole", r))
                            .selected_text(ro.name())
                            .width(cells[10].width() - 8.0)
                            .show_ui(ui, |ui| {
                                for x in Role::ALL {
                                    ui.selectable_value(&mut ro, x, x.name());
                                }
                            })
                            .response
                    });
                    if al != v.align || me != v.measure || ro != v.role {
                        v.align = al;
                        v.measure = me;
                        v.role = ro;
                        self.dirty = true;
                    }
                } else if r == nvars {
                    let id = Id::new("new_var_name");
                    let resp = ui.put(cells[0].shrink(2.0), egui::TextEdit::singleline(&mut self.new_var_name).id(id).frame(false).margin(egui::Margin::symmetric(4, 2)).hint_text("new name"));
                    if resp.lost_focus() && !self.new_var_name.trim().is_empty() {
                        let name = self.ds.unique_name(&self.new_var_name, None);
                        self.push_undo();
                        let v = crate::data::Variable::new_numeric(&name, self.ds.ncases);
                        self.ds.add_var(v);
                        self.new_var_name.clear();
                        self.status = format!("Added variable {}.", name);
                    }
                }
            }
        });
        if let Some((a, r)) = action {
            match a {
                0 => {
                    self.push_undo();
                    self.ds.insert_var(r);
                }
                1 => {
                    self.push_undo();
                    self.ds.delete_var(r);
                }
                2 => {
                    let labels = self.ds.vars[r].value_labels.clone();
                    self.vlabels = Some(ValueLabelsDlg { var: r, labels, value: String::new(), label: String::new(), sel: None });
                }
                3 => {
                    let m = self.ds.vars[r].missing.clone();
                    let mut vals = [String::new(), String::new(), String::new()];
                    for (i, x) in m.iter().take(3).enumerate() {
                        vals[i] = x.clone();
                    }
                    self.missing_dlg = Some(MissingDlg { var: r, on: !m.is_empty(), vals });
                }
                4 => self.sel = (r, 0),
                5 => {
                    self.push_undo();
                    let mut v = self.ds.vars[r].clone();
                    v.name = self.ds.unique_name(&format!("{}_copy", v.name), None);
                    self.ds.vars.insert(r + 1, v);
                }
                _ => {}
            }
        }
    }

    pub fn value_labels_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.vlabels else { return };
        if d.var >= self.ds.vars.len() {
            self.vlabels = None;
            return;
        }
        let numeric = self.ds.vars[d.var].is_numeric();
        let title = format!("Value Labels: {}", self.ds.vars[d.var].name);
        let mut done: Option<bool> = None;
        egui::Window::new(title).id(Id::new("vl_dialog")).collapsible(false).resizable(false).pivot(Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
            egui::Grid::new("vlg").num_columns(2).show(ui, |ui| {
                ui.label("Value:");
                ui.add(egui::TextEdit::singleline(&mut d.value).desired_width(120.0));
                ui.end_row();
                ui.label("Label:");
                ui.add(egui::TextEdit::singleline(&mut d.label).desired_width(260.0));
                ui.end_row();
            });
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    let key = crate::data::norm_key(&d.value, numeric);
                    let valid = !key.is_empty() && (!numeric || key.parse::<f64>().is_ok()) && !d.label.trim().is_empty();
                    if ui.add_enabled(valid, egui::Button::new("Add").min_size(Vec2::new(70.0, 24.0))).clicked() {
                        d.labels.retain(|(k, _)| *k != key);
                        d.labels.push((key.clone(), d.label.trim().to_string()));
                        crate::data::sort_keys(&mut [], numeric);
                        d.labels.sort_by(|a, b| {
                            if numeric {
                                a.0.parse::<f64>().unwrap_or(0.0).partial_cmp(&b.0.parse::<f64>().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal)
                            } else {
                                a.0.cmp(&b.0)
                            }
                        });
                        d.value.clear();
                        d.label.clear();
                        d.sel = None;
                    }
                    if ui.add_enabled(d.sel.is_some() && valid, egui::Button::new("Change").min_size(Vec2::new(70.0, 24.0))).clicked() {
                        if let Some(i) = d.sel {
                            if i < d.labels.len() {
                                d.labels[i] = (key, d.label.trim().to_string());
                            }
                        }
                    }
                    if ui.add_enabled(d.sel.is_some(), egui::Button::new("Remove").min_size(Vec2::new(70.0, 24.0))).clicked() {
                        if let Some(i) = d.sel {
                            if i < d.labels.len() {
                                d.labels.remove(i);
                            }
                        }
                        d.sel = None;
                    }
                });
                crate::dialogs::list_frame(ui, 280.0, 150.0, |ui| {
                    for (i, (k, l)) in d.labels.iter().enumerate() {
                        if ui.add_sized([272.0, 18.0], egui::SelectableLabel::new(d.sel == Some(i), format!("{} = \"{}\"", k, l))).clicked() {
                            d.sel = Some(i);
                            d.value = k.clone();
                            d.label = l.clone();
                        }
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_sized([74.0, 26.0], egui::Button::new("OK")).clicked() {
                    done = Some(true);
                }
                if ui.add_sized([74.0, 26.0], egui::Button::new("Cancel")).clicked() {
                    done = Some(false);
                }
            });
        });
        match done {
            Some(true) => {
                let d = self.vlabels.take().unwrap();
                self.push_undo();
                self.ds.vars[d.var].value_labels = d.labels;
            }
            Some(false) => self.vlabels = None,
            None => {}
        }
    }

    pub fn missing_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.missing_dlg else { return };
        if d.var >= self.ds.vars.len() {
            self.missing_dlg = None;
            return;
        }
        let title = format!("Missing Values: {}", self.ds.vars[d.var].name);
        let mut done: Option<bool> = None;
        egui::Window::new(title).id(Id::new("miss_dialog")).collapsible(false).resizable(false).pivot(Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
            ui.radio_value(&mut d.on, false, "No missing values");
            ui.radio_value(&mut d.on, true, "Discrete missing values");
            ui.add_enabled_ui(d.on, |ui| {
                ui.horizontal(|ui| {
                    for v in d.vals.iter_mut() {
                        ui.add(egui::TextEdit::singleline(v).desired_width(70.0));
                    }
                });
            });
            ui.label(RichText::new("Cases with these values are left out of analyses, like empty cells.").small());
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_sized([74.0, 26.0], egui::Button::new("OK")).clicked() {
                    done = Some(true);
                }
                if ui.add_sized([74.0, 26.0], egui::Button::new("Cancel")).clicked() {
                    done = Some(false);
                }
            });
        });
        match done {
            Some(true) => {
                let d = self.missing_dlg.take().unwrap();
                let numeric = self.ds.vars[d.var].is_numeric();
                self.push_undo();
                self.ds.vars[d.var].missing = if d.on { d.vals.iter().filter(|s| !s.trim().is_empty()).map(|s| crate::data::norm_key(s, numeric)).collect() } else { vec![] };
            }
            Some(false) => self.missing_dlg = None,
            None => {}
        }
    }

    pub fn goto_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.goto else { return };
        let mut done: Option<bool> = None;
        let title = if d.case { "Go To Case" } else { "Go To Variable" };
        egui::Window::new(title).id(Id::new("goto_dialog")).collapsible(false).resizable(false).pivot(Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut d.case, true, "Case");
                ui.selectable_value(&mut d.case, false, "Variable");
            });
            if d.case {
                ui.label("Go to case number:");
                let r = ui.add(egui::TextEdit::singleline(&mut d.text).desired_width(120.0));
                r.request_focus();
                if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    done = Some(true);
                }
            } else {
                ui.label("Go to variable:");
                let names: Vec<String> = self.ds.vars.iter().map(|v| v.display_name()).collect();
                egui::ComboBox::from_id_salt("goto_var").selected_text(names.get(d.var).cloned().unwrap_or_default()).width(260.0).show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut d.var, i, n);
                    }
                });
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_sized([74.0, 26.0], egui::Button::new("Go")).clicked() {
                    done = Some(true);
                }
                if ui.add_sized([74.0, 26.0], egui::Button::new("Close")).clicked() {
                    done = Some(false);
                }
            });
        });
        if let Some(go) = done {
            let d = self.goto.take().unwrap();
            if go {
                if d.case {
                    if let Ok(n) = d.text.trim().parse::<usize>() {
                        self.sel = (n.max(1) - 1, self.sel.1);
                        self.sel_mode = SelMode::Row;
                        self.top_row = self.sel.0.saturating_sub(3);
                    }
                } else if d.var < self.ds.vars.len() {
                    self.sel = (self.sel.0, d.var);
                    self.sel_mode = SelMode::Col;
                    self.left_col = d.var.saturating_sub(1);
                }
                self.view = View::Data;
            }
        }
    }

    pub fn find_ui(&mut self, ctx: &egui::Context) {
        let Some(d) = &mut self.find else { return };
        let mut next = false;
        let mut close = false;
        let col = self.sel.1;
        let col_name = self.ds.vars.get(col).map(|v| v.name.clone()).unwrap_or_else(|| "all variables".into());
        egui::Window::new("Find").id(Id::new("find_dialog")).collapsible(false).resizable(false).pivot(Align2::CENTER_CENTER).default_pos(ctx.screen_rect().center()).show(ctx, |ui| {
            ui.label(format!("Find in column: {}", col_name));
            let r = ui.add(egui::TextEdit::singleline(&mut d.text).desired_width(260.0));
            if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                next = true;
            }
            ui.checkbox(&mut d.whole, "Match the whole cell");
            if !d.msg.is_empty() {
                ui.label(RichText::new(&d.msg).color(Color32::from_rgb(160, 80, 0)));
            }
            ui.horizontal(|ui| {
                if ui.add_sized([90.0, 26.0], egui::Button::new("Find Next")).clicked() {
                    next = true;
                }
                if ui.add_sized([74.0, 26.0], egui::Button::new("Close")).clicked() {
                    close = true;
                }
            });
        });
        if next {
            let needle = d.text.trim().to_lowercase();
            let whole = d.whole;
            let n = self.ds.ncases;
            let mut found = None;
            if !needle.is_empty() && n > 0 && col < self.ds.vars.len() {
                let v = &self.ds.vars[col];
                for k in 1..=n {
                    let r = (self.sel.0 + k) % n;
                    let t = v.cell_text(r, self.labels_on).to_lowercase();
                    let raw = v.edit_text(r).to_lowercase();
                    let hit = if whole { t == needle || raw == needle } else { t.contains(&needle) || raw.contains(&needle) };
                    if hit {
                        found = Some(r);
                        break;
                    }
                }
            }
            if let Some(f) = &mut self.find {
                match found {
                    Some(r) => {
                        self.sel = (r, col);
                        self.sel_mode = SelMode::Cell;
                        self.top_row = r.saturating_sub(3);
                        f.msg.clear();
                    }
                    None => f.msg = "Not found.".into(),
                }
            }
        }
        if close {
            self.find = None;
        }
    }

    // ------------------------------------------------------------ output viewer

    pub fn output_viewport(&mut self, ctx: &egui::Context) {
        let id = egui::ViewportId::from_hash_of("output_viewer");
        let builder = egui::ViewportBuilder::default().with_title(format!("Output1 [Document1] - {} Viewer", APP_NAME)).with_inner_size([1060.0, 720.0]).with_min_inner_size([480.0, 320.0]);
        if self.out_focus {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            self.out_focus = false;
        }
        ctx.show_viewport_immediate(id, builder, |ctx, class| {
            if class == egui::ViewportClass::Embedded {
                let mut open = true;
                egui::Window::new("Output Viewer").open(&mut open).default_size([760.0, 520.0]).show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| self.output_content(ui));
                });
                if !open {
                    self.out_open = false;
                }
                return;
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                self.out_open = false;
            }
            self.output_ui(ctx);
            if let (Some(path), Some(img)) = (self.shot.clone(), crate::screenshot_event(ctx)) {
                let _ = crate::save_png(&path, &img);
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
            }
        });
    }

    pub fn output_ui(&mut self, ctx: &egui::Context) {
        let mut export_html = false;
        let mut export_txt = false;
        let mut print = false;
        egui::TopBottomPanel::top("out_menu").frame(egui::Frame::default().fill(Color32::from_rgb(250, 250, 251)).inner_margin(egui::Margin::symmetric(4, 2))).show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Export as Web Page (HTML)...").clicked() {
                        export_html = true;
                        ui.close_menu();
                    }
                    if ui.button("Export as Text...").clicked() {
                        export_txt = true;
                        ui.close_menu();
                    }
                    if ui.button("Print Preview (opens in browser)").clicked() {
                        print = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.out_open = false;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.add_enabled(self.out_sel.is_some(), egui::Button::new("Copy")).clicked() {
                        if let Some(t) = self.selected_output_text() {
                            ui.ctx().copy_text(t);
                        }
                        ui.close_menu();
                    }
                    if ui.add_enabled(self.out_sel.is_some(), egui::Button::new("Delete")).clicked() {
                        self.delete_selected_output();
                        ui.close_menu();
                    }
                    if ui.button("Clear All Output").clicked() {
                        self.out.clear();
                        self.out_sel = None;
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Data Editor").clicked() {
                        ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                        ui.close_menu();
                    }
                });
                let had_dialog = self.dialog.is_some();
                self.analysis_menus(ui);
                if !had_dialog && self.dialog.is_some() {
                    ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                }
                ui.menu_button("Window", |ui| {
                    if ui.button("Data Editor").clicked() {
                        ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                        ui.close_menu();
                    }
                    if ui.button("Syntax Editor").clicked() {
                        self.syn_open = true;
                        self.syn_focus = true;
                        ui.close_menu();
                    }
                });
            });
        });
        egui::TopBottomPanel::top("out_tools").frame(egui::Frame::default().fill(Color32::from_rgb(243, 245, 248)).inner_margin(egui::Margin::symmetric(6, 3)).stroke(Stroke::new(1.0, Color32::from_gray(215)))).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                if tool(ui, Icon::Save, "Export as a web page", !self.out.is_empty()).clicked() {
                    export_html = true;
                }
                if tool(ui, Icon::Export, "Print preview (opens in browser)", !self.out.is_empty()).clicked() {
                    print = true;
                }
                tool_sep(ui);
                if tool(ui, Icon::NewData, "Go to the data editor", true).clicked() {
                    ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                }
                if tool(ui, Icon::Recall, "Recall recently used dialogs", self.last_dialog.is_some()).clicked() {
                    if let Some(d) = self.last_dialog.clone() {
                        self.dialog = Some(d);
                        ui.ctx().send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
                    }
                }
                if tool(ui, Icon::Syntax, "Syntax editor", true).clicked() {
                    self.syn_open = true;
                    self.syn_focus = true;
                }
                tool_sep(ui);
                ui.label(RichText::new(format!("{} items", self.out.len())).size(12.0).color(Color32::from_gray(90)));
            });
        });
        egui::TopBottomPanel::bottom("out_status").frame(egui::Frame::default().fill(Color32::from_rgb(236, 238, 241)).inner_margin(egui::Margin::symmetric(8, 2))).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} Processor is ready", APP_NAME)).size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Unicode:ON").size(12.0));
                });
            });
        });
        egui::SidePanel::left("outline").default_width(240.0).frame(egui::Frame::default().fill(Color32::from_rgb(248, 249, 251)).inner_margin(egui::Margin::same(6))).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new("Output").strong());
                let mut click: Option<(usize, Option<usize>)> = None;
                for (bi, b) in self.out.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        let sel = self.out_sel == Some((bi, None));
                        if ui.selectable_label(sel, RichText::new(&b.name).strong()).clicked() {
                            click = Some((bi, None));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(26.0);
                        if ui.selectable_label(false, RichText::new("Log").color(Color32::from_gray(90))).clicked() {
                            click = Some((bi, None));
                        }
                    });
                    for (pi, p) in b.parts.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add_space(26.0);
                            let sel = self.out_sel == Some((bi, Some(pi)));
                            let label = match p {
                                Part::Warn(_) => RichText::new(p.title()).color(Color32::from_rgb(180, 90, 0)),
                                _ => RichText::new(p.title()),
                            };
                            if ui.selectable_label(sel, label).clicked() {
                                click = Some((bi, Some(pi)));
                            }
                        });
                    }
                }
                if let Some(c) = click {
                    self.out_sel = Some(c);
                    self.out_scroll = Some(c);
                }
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::default().fill(Color32::WHITE).inner_margin(egui::Margin::same(16))).show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| self.output_content(ui));
        });
        if ctx.input(|i| i.key_pressed(Key::Delete)) && !ctx.wants_keyboard_input() {
            self.delete_selected_output();
        }
        if ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy))) {
            if let Some(t) = self.selected_output_text() {
                ctx.copy_text(t);
            }
        }
        if export_html || print {
            let html = output::blocks_html(&self.out);
            if print {
                let p = std::env::temp_dir().join("statdesk_output_preview.html");
                if std::fs::write(&p, html).is_ok() {
                    open_in_browser(&p);
                }
            } else if let Some(p) = rfd::FileDialog::new().set_title("Export Output").set_file_name("output.html").add_filter("Web page (*.html)", &["html", "htm"]).save_file() {
                match std::fs::write(&p, html) {
                    Ok(()) => self.status = format!("Exported output to {}.", p.display()),
                    Err(e) => self.message = Some(("Couldn't export".into(), e.to_string())),
                }
            }
        }
        if export_txt {
            if let Some(p) = rfd::FileDialog::new().set_title("Export Output as Text").set_file_name("output.txt").add_filter("Text (*.txt)", &["txt"]).save_file() {
                match std::fs::write(&p, output::blocks_text(&self.out)) {
                    Ok(()) => self.status = format!("Exported output to {}.", p.display()),
                    Err(e) => self.message = Some(("Couldn't export".into(), e.to_string())),
                }
            }
        }
    }

    fn selected_output_text(&self) -> Option<String> {
        let (bi, pi) = self.out_sel?;
        let b = self.out.get(bi)?;
        Some(match pi {
            None => output::blocks_text(std::slice::from_ref(b)),
            Some(pi) => match b.parts.get(pi)? {
                Part::Table(t) => t.to_tsv(),
                Part::Chart(c) => output::chart_svg(c),
                Part::Text(t) | Part::Warn(t) => t.clone(),
            },
        })
    }

    fn delete_selected_output(&mut self) {
        if let Some((bi, pi)) = self.out_sel.take() {
            if bi < self.out.len() {
                match pi {
                    None => {
                        self.out.remove(bi);
                    }
                    Some(pi) => {
                        if pi < self.out[bi].parts.len() {
                            self.out[bi].parts.remove(pi);
                        }
                        if self.out[bi].parts.is_empty() {
                            self.out.remove(bi);
                        }
                    }
                }
            }
        }
    }

    fn output_content(&mut self, ui: &mut egui::Ui) {
        if self.out.is_empty() {
            ui.add_space(30.0);
            ui.label(RichText::new("Results of analyses appear here.").size(15.0).color(Color32::from_gray(110)));
            ui.label(RichText::new("Pick a procedure from the Analyze or Graphs menu.").color(Color32::from_gray(130)));
            return;
        }
        let mut click: Option<(usize, Option<usize>)> = None;
        let mut del: Option<(usize, Option<usize>)> = None;
        let scroll = self.out_scroll.take();
        for (bi, b) in self.out.iter().enumerate() {
            ui.add_space(10.0);
            let head = ui.label(RichText::new(&b.name).size(19.0).color(Color32::from_rgb(20, 40, 80)));
            if scroll == Some((bi, None)) {
                head.scroll_to_me(Some(egui::Align::TOP));
            }
            if head.interact(Sense::click()).clicked() {
                click = Some((bi, None));
            }
            if !b.syntax.is_empty() {
                egui::Frame::default().fill(Color32::from_rgb(247, 248, 250)).stroke(Stroke::new(1.0, Color32::from_gray(225))).inner_margin(egui::Margin::symmetric(8, 5)).show(ui, |ui| {
                    ui.label(RichText::new(&b.syntax).monospace().size(12.0).color(Color32::from_gray(70)));
                });
            }
            for (pi, p) in b.parts.iter().enumerate() {
                ui.add_space(8.0);
                let sel = self.out_sel == Some((bi, Some(pi)));
                let resp = match p {
                    Part::Table(t) => output::table_ui(ui, t, sel),
                    Part::Chart(c) => output::chart_ui(ui, c, sel),
                    Part::Text(t) => ui.label(RichText::new(t).color(Color32::from_gray(50))).interact(Sense::click()),
                    Part::Warn(t) => {
                        egui::Frame::default()
                            .fill(Color32::from_rgb(255, 246, 224))
                            .stroke(Stroke::new(1.0, Color32::from_rgb(232, 200, 120)))
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Warning").strong().color(Color32::from_rgb(150, 80, 0)));
                                ui.label(t);
                            })
                            .response
                            .interact(Sense::click())
                    }
                };
                if scroll == Some((bi, Some(pi))) {
                    resp.scroll_to_me(Some(egui::Align::TOP));
                }
                if resp.clicked() {
                    click = Some((bi, Some(pi)));
                }
                resp.context_menu(|ui| {
                    if ui.button("Copy").clicked() {
                        let t = match p {
                            Part::Table(t) => t.to_tsv(),
                            Part::Chart(c) => output::chart_svg(c),
                            Part::Text(t) | Part::Warn(t) => t.clone(),
                        };
                        ui.ctx().copy_text(t);
                        ui.close_menu();
                    }
                    if let Part::Table(t) = p {
                        if ui.button("Copy as HTML").clicked() {
                            ui.ctx().copy_text(t.to_html());
                            ui.close_menu();
                        }
                    }
                    if ui.button("Delete").clicked() {
                        del = Some((bi, Some(pi)));
                        ui.close_menu();
                    }
                });
            }
            ui.add_space(12.0);
            ui.separator();
        }
        if let Some(c) = click {
            self.out_sel = Some(c);
        }
        if let Some(d) = del {
            self.out_sel = Some(d);
            self.delete_selected_output();
        }
    }

    // ------------------------------------------------------------ syntax editor

    pub fn syntax_viewport(&mut self, ctx: &egui::Context) {
        let id = egui::ViewportId::from_hash_of("syntax_editor");
        let title = format!("{} - {} Syntax Editor", self.syn_path.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Syntax1".into()), APP_NAME);
        let builder = egui::ViewportBuilder::default().with_title(title).with_inner_size([820.0, 560.0]).with_min_inner_size([420.0, 260.0]);
        if self.syn_focus {
            ctx.send_viewport_cmd_to(id, egui::ViewportCommand::Focus);
            self.syn_focus = false;
        }
        ctx.show_viewport_immediate(id, builder, |ctx, class| {
            if class == egui::ViewportClass::Embedded {
                let mut open = true;
                egui::Window::new("Syntax Editor").open(&mut open).default_size([640.0, 420.0]).show(ctx, |ui| {
                    if ui.button("Run All").clicked() {
                        let t = self.syn_text.clone();
                        self.run_script(&t);
                    }
                    ui.add(egui::TextEdit::multiline(&mut self.syn_text).code_editor().desired_width(f32::INFINITY).desired_rows(18));
                });
                if !open {
                    self.syn_open = false;
                }
                return;
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                self.syn_open = false;
            }
            self.syntax_ui(ctx);
        });
    }

    fn syntax_ui(&mut self, ctx: &egui::Context) {
        let mut run: Option<bool> = None; // Some(true) = all, Some(false) = selection/current
        let mut save_as = false;
        let mut save = false;
        egui::TopBottomPanel::top("syn_menu").frame(egui::Frame::default().fill(Color32::from_rgb(250, 250, 251)).inner_margin(egui::Margin::symmetric(4, 2))).show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.syn_text.clear();
                        self.syn_path = None;
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        ui.close_menu();
                        if let Some(p) = rfd::FileDialog::new().add_filter("Syntax (*.sds, *.sps, *.txt)", &["sds", "sps", "txt"]).pick_file() {
                            if let Ok(t) = std::fs::read_to_string(&p) {
                                self.syn_text = t;
                                self.syn_path = Some(p);
                            }
                        }
                    }
                    if ui.button("Save").clicked() {
                        save = true;
                        ui.close_menu();
                    }
                    if ui.button("Save As...").clicked() {
                        save_as = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.syn_open = false;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Run", |ui| {
                    if ui.button("All").clicked() {
                        run = Some(true);
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Selection / Current Command").shortcut_text("Ctrl+R")).clicked() {
                        run = Some(false);
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    ui.set_max_width(380.0);
                    ui.label(crate::syntax::SYNTAX_HELP);
                });
            });
        });
        egui::TopBottomPanel::top("syn_tools").frame(egui::Frame::default().fill(Color32::from_rgb(243, 245, 248)).inner_margin(egui::Margin::symmetric(6, 3)).stroke(Stroke::new(1.0, Color32::from_gray(215)))).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if tool(ui, Icon::Run, "Run selection (or the command at the cursor)", true).clicked() {
                    run = Some(false);
                }
                if ui.button("Run All").clicked() {
                    run = Some(true);
                }
                tool_sep(ui);
                if tool(ui, Icon::Output, "Show output", true).clicked() {
                    self.out_open = true;
                    self.out_focus = true;
                }
            });
        });
        egui::TopBottomPanel::bottom("syn_status").frame(egui::Frame::default().fill(Color32::from_rgb(236, 238, 241)).inner_margin(egui::Margin::symmetric(8, 2))).show(ctx, |ui| {
            let line = self.syn_text[..self.syn_cursor.0.min(self.syn_text.len())].matches('\n').count() + 1;
            ui.label(RichText::new(format!("Ln {}    {} commands", line, crate::syntax::split_commands(&self.syn_text).len())).size(12.0));
        });
        egui::CentralPanel::default().frame(egui::Frame::default().fill(Color32::WHITE).inner_margin(egui::Margin::same(4))).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let out = egui::TextEdit::multiline(&mut self.syn_text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .hint_text("Type commands here, for example:\nDESCRIPTIVES VARIABLES=age salary.\nFREQUENCIES VARIABLES=gender /BARCHART.")
                    .show(ui);
                if let Some(cr) = out.cursor_range {
                    let a = char_to_byte(&self.syn_text, cr.primary.ccursor.index);
                    let b = char_to_byte(&self.syn_text, cr.secondary.ccursor.index);
                    self.syn_cursor = (a.min(b), a.max(b));
                }
            });
        });
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::R)) {
            run = Some(false);
        }
        if save || save_as {
            let path = if save && self.syn_path.is_some() { self.syn_path.clone() } else { rfd::FileDialog::new().set_file_name("syntax.sds").add_filter("Syntax (*.sds, *.sps)", &["sds", "sps"]).save_file() };
            if let Some(p) = path {
                match std::fs::write(&p, &self.syn_text) {
                    Ok(()) => self.syn_path = Some(p),
                    Err(e) => self.message = Some(("Couldn't save the syntax".into(), e.to_string())),
                }
            }
        }
        if let Some(all) = run {
            let text = if all {
                self.syn_text.clone()
            } else {
                let (a, b) = self.syn_cursor;
                if b > a {
                    self.syn_text[a..b].to_string()
                } else {
                    // the command around the cursor
                    let cmds = crate::syntax::split_commands(&self.syn_text);
                    let mut pick = String::new();
                    for (start, c) in cmds {
                        if start <= a.max(1) - 0 || pick.is_empty() {
                            if start <= a || pick.is_empty() {
                                pick = c;
                            }
                        }
                    }
                    format!("{}.", pick)
                }
            };
            self.run_script(&text);
        }
    }
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

fn open_in_browser(p: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", ""]).arg(p).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(p).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(p).spawn();
    }
}

// ---------------------------------------------------------------- toolbar icons

#[derive(Clone, Copy)]
pub enum Icon {
    Open,
    Save,
    Export,
    Recall,
    Undo,
    Redo,
    GotoCase,
    GotoVar,
    Variables,
    Find,
    InsertCase,
    InsertVar,
    Select,
    ValueLabels,
    Chart,
    Stats,
    Output,
    Syntax,
    NewData,
    Run,
}

pub fn tool_sep(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 24.0), Sense::hover());
    ui.painter().line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.0, Color32::from_gray(200)));
    ui.add_space(4.0);
}

pub fn tool(ui: &mut egui::Ui, icon: Icon, tip: &str, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 28.0), if enabled { Sense::click() } else { Sense::hover() });
    if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, Color32::from_rgb(222, 232, 246));
        ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::from_rgb(170, 195, 230)), StrokeKind::Inside);
    }
    draw_icon(ui.painter(), rect.shrink(5.0), icon, enabled);
    resp.on_hover_text(tip)
}

fn grid_icon(p: &egui::Painter, r: Rect, ink: Color32) {
    p.rect_filled(r, 1.0, Color32::WHITE);
    p.rect_stroke(r, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
    for k in 1..3 {
        let x = r.min.x + r.width() * k as f32 / 3.0;
        let y = r.min.y + r.height() * k as f32 / 3.0;
        p.line_segment([Pos2::new(x, r.min.y), Pos2::new(x, r.max.y)], Stroke::new(0.7, ink));
        p.line_segment([Pos2::new(r.min.x, y), Pos2::new(r.max.x, y)], Stroke::new(0.7, ink));
    }
}

fn arc(c: Pos2, rad: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    (0..=12).map(|k| {
        let t = a0 + (a1 - a0) * k as f32 / 12.0;
        Pos2::new(c.x + rad * t.cos(), c.y + rad * t.sin())
    }).collect()
}

pub fn draw_icon(p: &egui::Painter, r: Rect, icon: Icon, enabled: bool) {
    let dim = |c: Color32| if enabled { c } else { Color32::from_gray(190) };
    let ink = dim(Color32::from_rgb(60, 70, 90));
    let blue = dim(Color32::from_rgb(40, 100, 190));
    let c = r.center();
    let (w, h) = (r.width(), r.height());
    match icon {
        Icon::Open => {
            let pts = vec![Pos2::new(r.min.x, r.min.y + 3.0), Pos2::new(r.min.x + 6.0, r.min.y + 3.0), Pos2::new(r.min.x + 8.0, r.min.y + 5.0), Pos2::new(r.max.x - 1.0, r.min.y + 5.0), Pos2::new(r.max.x - 1.0, r.max.y - 1.0), Pos2::new(r.min.x, r.max.y - 1.0)];
            p.add(egui::Shape::convex_polygon(vec![pts[0], pts[1], pts[2], pts[3], pts[4], pts[5]], dim(Color32::from_rgb(240, 196, 80)), Stroke::new(1.0, dim(Color32::from_rgb(170, 120, 30)))));
            p.add(egui::Shape::convex_polygon(vec![Pos2::new(r.min.x + 3.0, r.min.y + 8.0), Pos2::new(r.max.x + 1.0, r.min.y + 8.0), Pos2::new(r.max.x - 2.0, r.max.y - 1.0), Pos2::new(r.min.x, r.max.y - 1.0)], dim(Color32::from_rgb(252, 216, 112)), Stroke::new(1.0, dim(Color32::from_rgb(170, 120, 30)))));
        }
        Icon::Save => {
            p.rect_filled(r, 2.0, blue);
            p.rect_filled(Rect::from_min_size(Pos2::new(r.min.x + 3.0, r.min.y + 1.0), Vec2::new(w - 6.0, h * 0.4)), 0.0, Color32::WHITE);
            p.rect_filled(Rect::from_min_size(Pos2::new(r.min.x + 4.0, r.max.y - h * 0.35), Vec2::new(w - 8.0, h * 0.35 - 1.0)), 0.0, dim(Color32::from_rgb(200, 215, 235)));
        }
        Icon::Export => {
            p.rect_filled(Rect::from_min_max(Pos2::new(r.min.x + 3.0, r.min.y), Pos2::new(r.max.x - 3.0, r.min.y + 6.0)), 0.0, Color32::WHITE);
            p.rect_stroke(Rect::from_min_max(Pos2::new(r.min.x + 3.0, r.min.y), Pos2::new(r.max.x - 3.0, r.min.y + 6.0)), 0.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            p.rect_filled(Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 6.0), Pos2::new(r.max.x, r.max.y - 4.0)), 2.0, dim(Color32::from_rgb(120, 130, 145)));
            p.rect_filled(Rect::from_min_max(Pos2::new(r.min.x + 3.0, r.max.y - 6.0), Pos2::new(r.max.x - 3.0, r.max.y)), 0.0, Color32::WHITE);
            p.rect_stroke(Rect::from_min_max(Pos2::new(r.min.x + 3.0, r.max.y - 6.0), Pos2::new(r.max.x - 3.0, r.max.y)), 0.0, Stroke::new(1.0, ink), StrokeKind::Inside);
        }
        Icon::Recall => {
            let win = Rect::from_min_max(r.min, Pos2::new(r.max.x - 2.0, r.max.y - 2.0));
            p.rect_filled(win, 1.0, Color32::WHITE);
            p.rect_stroke(win, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            p.rect_filled(Rect::from_min_size(win.min, Vec2::new(win.width(), 4.0)), 0.0, blue);
            for k in 0..3 {
                let y = win.min.y + 7.0 + k as f32 * 3.0;
                p.line_segment([Pos2::new(win.min.x + 3.0, y), Pos2::new(win.max.x - 6.0, y)], Stroke::new(1.0, ink));
            }
        }
        Icon::Undo | Icon::Redo => {
            let flip = matches!(icon, Icon::Redo);
            let pts: Vec<Pos2> = arc(Pos2::new(c.x, c.y + 2.0), w * 0.38, std::f32::consts::PI * 1.05, std::f32::consts::PI * 2.15).into_iter().map(|q| if flip { Pos2::new(2.0 * c.x - q.x, q.y) } else { q }).collect();
            let start = pts[0];
            p.add(egui::Shape::line(pts, Stroke::new(2.0, blue)));
            let s = if flip { -1.0 } else { 1.0 };
            p.add(egui::Shape::convex_polygon(vec![Pos2::new(start.x - 4.0 * s, start.y - 3.0), Pos2::new(start.x + 4.0 * s, start.y - 3.0), Pos2::new(start.x, start.y + 4.0)], blue, Stroke::NONE));
        }
        Icon::GotoCase | Icon::InsertCase => {
            let g = Rect::from_min_max(Pos2::new(r.min.x + 4.0, r.min.y), r.max);
            grid_icon(p, g, ink);
            p.rect_filled(Rect::from_min_size(Pos2::new(g.min.x + 1.0, g.min.y + g.height() / 3.0), Vec2::new(g.width() - 2.0, g.height() / 3.0)), 0.0, dim(Color32::from_rgba_unmultiplied(40, 100, 190, 120)));
            if matches!(icon, Icon::GotoCase) {
                p.add(egui::Shape::convex_polygon(vec![Pos2::new(r.min.x + 4.0, c.y), Pos2::new(r.min.x - 1.0, c.y - 4.0), Pos2::new(r.min.x - 1.0, c.y + 4.0)], blue, Stroke::NONE));
            } else {
                p.text(Pos2::new(r.min.x + 1.0, r.min.y + 2.0), Align2::CENTER_CENTER, "+", FontId::proportional(12.0), dim(Color32::from_rgb(30, 140, 60)));
            }
        }
        Icon::GotoVar | Icon::InsertVar => {
            let g = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 4.0), r.max);
            grid_icon(p, g, ink);
            p.rect_filled(Rect::from_min_size(Pos2::new(g.min.x + g.width() / 3.0, g.min.y + 1.0), Vec2::new(g.width() / 3.0, g.height() - 2.0)), 0.0, dim(Color32::from_rgba_unmultiplied(40, 100, 190, 120)));
            if matches!(icon, Icon::GotoVar) {
                p.add(egui::Shape::convex_polygon(vec![Pos2::new(c.x, r.min.y + 4.0), Pos2::new(c.x - 4.0, r.min.y - 1.0), Pos2::new(c.x + 4.0, r.min.y - 1.0)], blue, Stroke::NONE));
            } else {
                p.text(Pos2::new(r.max.x - 1.0, r.min.y + 1.0), Align2::CENTER_CENTER, "+", FontId::proportional(12.0), dim(Color32::from_rgb(30, 140, 60)));
            }
        }
        Icon::Variables => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(r, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            for k in 0..4 {
                let y = r.min.y + 3.0 + k as f32 * 3.5;
                p.rect_filled(Rect::from_min_size(Pos2::new(r.min.x + 2.5, y - 1.0), Vec2::new(2.5, 2.5)), 0.0, blue);
                p.line_segment([Pos2::new(r.min.x + 6.5, y), Pos2::new(r.max.x - 2.5, y)], Stroke::new(1.0, ink));
            }
        }
        Icon::Find => {
            p.circle_stroke(Pos2::new(c.x - 2.0, c.y - 2.0), w * 0.3, Stroke::new(2.0, ink));
            p.line_segment([Pos2::new(c.x + 2.5, c.y + 2.5), Pos2::new(r.max.x, r.max.y)], Stroke::new(3.0, ink));
        }
        Icon::Select => {
            p.add(egui::Shape::convex_polygon(vec![Pos2::new(r.min.x, r.min.y + 1.0), Pos2::new(r.max.x, r.min.y + 1.0), Pos2::new(c.x + 2.0, c.y + 1.0), Pos2::new(c.x - 2.0, c.y + 1.0)], dim(Color32::from_rgb(90, 150, 220)), Stroke::new(1.0, blue)));
            p.rect_filled(Rect::from_min_max(Pos2::new(c.x - 2.0, c.y + 1.0), Pos2::new(c.x + 2.0, r.max.y)), 0.0, blue);
        }
        Icon::ValueLabels => {
            let pts = vec![Pos2::new(r.min.x, c.y), Pos2::new(r.min.x + 5.0, r.min.y + 2.0), Pos2::new(r.max.x, r.min.y + 2.0), Pos2::new(r.max.x, r.max.y - 2.0), Pos2::new(r.min.x + 5.0, r.max.y - 2.0)];
            p.add(egui::Shape::convex_polygon(pts, dim(Color32::from_rgb(255, 236, 170)), Stroke::new(1.0, dim(Color32::from_rgb(170, 130, 40)))));
            p.text(Pos2::new(c.x + 2.0, c.y), Align2::CENTER_CENTER, "A", FontId::proportional(10.0), ink);
        }
        Icon::Chart => {
            p.line_segment([Pos2::new(r.min.x, r.min.y), Pos2::new(r.min.x, r.max.y)], Stroke::new(1.0, ink));
            p.line_segment([Pos2::new(r.min.x, r.max.y), Pos2::new(r.max.x, r.max.y)], Stroke::new(1.0, ink));
            for (k, hh) in [0.45f32, 0.85, 0.65, 0.3].iter().enumerate() {
                let x = r.min.x + 2.0 + k as f32 * (w - 3.0) / 4.0;
                p.rect_filled(Rect::from_min_max(Pos2::new(x, r.max.y - h * hh), Pos2::new(x + (w - 3.0) / 4.0 - 1.0, r.max.y - 1.0)), 0.0, blue);
            }
        }
        Icon::Stats => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.rect_stroke(r, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            p.text(c, Align2::CENTER_CENTER, "x̄", FontId::proportional(13.0), blue);
        }
        Icon::Output => {
            let d = Rect::from_min_max(Pos2::new(r.min.x + 2.0, r.min.y), Pos2::new(r.max.x - 2.0, r.max.y));
            p.rect_filled(d, 1.0, Color32::WHITE);
            p.rect_stroke(d, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            for (k, hh) in [0.3f32, 0.55, 0.4].iter().enumerate() {
                let x = d.min.x + 2.0 + k as f32 * 3.5;
                p.rect_filled(Rect::from_min_max(Pos2::new(x, d.max.y - 2.0 - h * hh), Pos2::new(x + 2.5, d.max.y - 2.0)), 0.0, dim(Color32::from_rgb(220, 120, 40)));
            }
            p.line_segment([Pos2::new(d.min.x + 2.0, d.min.y + 3.0), Pos2::new(d.max.x - 2.0, d.min.y + 3.0)], Stroke::new(1.0, ink));
        }
        Icon::Syntax => {
            let d = Rect::from_min_max(Pos2::new(r.min.x + 2.0, r.min.y), Pos2::new(r.max.x - 2.0, r.max.y));
            p.rect_filled(d, 1.0, Color32::WHITE);
            p.rect_stroke(d, 1.0, Stroke::new(1.0, ink), StrokeKind::Inside);
            for k in 0..4 {
                let y = d.min.y + 3.0 + k as f32 * 3.2;
                let len = [0.8f32, 0.55, 0.7, 0.4][k];
                p.line_segment([Pos2::new(d.min.x + 2.0, y), Pos2::new(d.min.x + 2.0 + (d.width() - 4.0) * len, y)], Stroke::new(1.0, if k == 0 { blue } else { ink }));
            }
        }
        Icon::NewData => {
            grid_icon(p, r, ink);
            p.rect_filled(Rect::from_min_size(r.min + Vec2::new(1.0, 1.0), Vec2::new(w - 2.0, h / 3.0 - 1.0)), 0.0, dim(Color32::from_rgb(214, 228, 246)));
        }
        Icon::Run => {
            p.add(egui::Shape::convex_polygon(vec![Pos2::new(r.min.x + 2.0, r.min.y), Pos2::new(r.max.x, c.y), Pos2::new(r.min.x + 2.0, r.max.y)], dim(Color32::from_rgb(40, 160, 70)), Stroke::new(1.0, dim(Color32::from_rgb(20, 110, 40)))));
        }
    }
}

pub fn paint_logo(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 8.0, Color32::from_rgb(34, 92, 170));
    let base = r.max.y - 10.0;
    for (k, hh) in [0.35f32, 0.6, 0.45, 0.75].iter().enumerate() {
        let x = r.min.x + 9.0 + k as f32 * 10.0;
        p.rect_filled(Rect::from_min_max(Pos2::new(x, base - (r.height() - 20.0) * hh), Pos2::new(x + 7.0, base)), 1.0, Color32::WHITE);
    }
}

pub fn paint_welcome_art(p: &egui::Painter, r: Rect) {
    let top = Color32::from_rgb(34, 92, 170);
    p.rect_filled(r, 6.0, top);
    let mut pts = vec![];
    for k in 0..=40 {
        let t = k as f32 / 40.0;
        let z = (t - 0.5) * 6.0;
        let y = r.max.y - 18.0 - (-0.5 * z * z).exp() * (r.height() - 44.0);
        pts.push(Pos2::new(r.min.x + 14.0 + t * (r.width() - 28.0), y));
    }
    for k in 0..12 {
        let t = (k as f32 + 0.5) / 12.0;
        let z = (t - 0.5) * 6.0;
        let hh = (-0.5 * z * z).exp() * (r.height() - 44.0) * 0.92;
        let x = r.min.x + 14.0 + t * (r.width() - 28.0);
        let bw = (r.width() - 28.0) / 12.0 - 3.0;
        p.rect_filled(Rect::from_min_max(Pos2::new(x - bw / 2.0, r.max.y - 18.0 - hh), Pos2::new(x + bw / 2.0, r.max.y - 18.0)), 1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 70));
    }
    p.add(egui::Shape::line(pts, Stroke::new(2.0, Color32::WHITE)));
    p.text(Pos2::new(r.min.x + 12.0, r.min.y + 14.0), Align2::LEFT_CENTER, APP_NAME, FontId::proportional(16.0), Color32::WHITE);
}

pub fn welcome_item(ui: &mut egui::Ui, icon: Icon, title: &str, sub: &str) -> egui::Response {
    let w = ui.available_width().min(292.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(rect, 3.0, Color32::from_rgb(228, 238, 250));
    }
    draw_icon(p, Rect::from_min_size(rect.min + Vec2::new(6.0, 8.0), Vec2::new(18.0, 18.0)), icon, true);
    let pc = ui.painter_at(rect);
    pc.text(Pos2::new(rect.min.x + 32.0, rect.min.y + 11.0), Align2::LEFT_CENTER, title, FontId::proportional(13.0), Color32::from_gray(20));
    pc.text(Pos2::new(rect.min.x + 32.0, rect.min.y + 25.0), Align2::LEFT_CENTER, sub, FontId::proportional(11.0), Color32::from_gray(110));
    resp
}

pub const HELP: &[(&str, &str)] = &[
    ("Data View", "Each row is a case and each column is a variable. Click a cell and type to enter a value; Enter moves down, Tab moves right. Typing in an empty column creates a new variable. Paste blocks copied from a spreadsheet with Ctrl+V. Right-click a column heading to sort, insert or delete."),
    ("Variable View", "Each row describes one variable: its name, type (numeric or string), width, decimals, label, value labels, missing values, display width, alignment, measurement level and role. Click Values or Missing to edit them."),
    ("Value labels", "Give codes meaningful names (for example 1 = Male, 2 = Female). Turn on View > Value Labels or the tag button to show labels in the Data View; you can also type a label into a cell."),
    ("Analyses", "Analyze has Frequencies, Descriptives, Crosstabs, Means, t tests, one-way ANOVA, correlations, linear regression, reliability and nonparametric tests. Move variables into the boxes with the arrow buttons (or double-click), set options, and press OK. Results appear in the Output Viewer."),
    ("Charts", "Graphs makes bar charts, histograms, scatterplots, boxplots and pie charts. They appear in the Output Viewer and are exported with it."),
    ("Transform and Data", "Compute Variable creates or replaces a variable from an expression such as (pretest + posttest) / 2 or MEAN(q1, q2, q3). Select Cases filters cases by a condition; unselected rows are crossed out and left out of analyses. Sort Cases reorders rows."),
    ("Syntax", "Every dialog's Paste button writes its command into the Syntax Editor, where you can edit and run it again (Run > All, or Ctrl+R for the command at the cursor). Commands end with a period."),
    ("Files", "Open CSV or tab-delimited text (the first row becomes variable names) or StatDesk data (.sdv) files, which keep labels and all variable properties. Save As can write .sdv, CSV or tab-delimited files. Drop a file on the window to open it."),
    ("Output", "The outline on the left lists every result; click to jump to it. Right-click a table to copy it (it pastes into spreadsheets and documents). File > Export saves the whole output as a web page with tables and charts."),
];
