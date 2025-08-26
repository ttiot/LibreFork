use adw::prelude::*;
use gtk::Orientation;
use gtk4 as gtk;
use gtk4::{cairo, pango};
use librefork_core::CommitInfo;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct CommitList {
    root: gtk::Box,
    list: gtk::ListBox,
    commits: Rc<RefCell<Vec<CommitInfo>>>,
    on_select: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
}

#[derive(Clone)]
struct GraphRowData {
    active_before: Vec<Option<String>>,
    node_lane: usize,
    parent_lanes: Vec<usize>,
    lane_count: usize,
}

impl CommitList {
    pub fn new() -> Self {
        let root = gtk::Box::new(Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_vexpand(true);
        let list = gtk::ListBox::new();
        list.add_css_class("boxed-list");
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.set_vexpand(true);

        root.append(&list);

        let on_select: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));
        let commits: Rc<RefCell<Vec<CommitInfo>>> = Rc::new(RefCell::new(Vec::new()));

        let cb = on_select.clone();
        list.connect_row_selected(move |_, row| {
            if let (Some(row), Some(cb)) = (row, &*cb.borrow()) {
                let oid = row.widget_name().to_string();
                if !oid.is_empty() {
                    cb(&oid);
                }
            }
        });

        Self {
            root,
            list,
            commits,
            on_select,
        }
    }

    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    fn row_from_commit(c: &CommitInfo, g: &GraphRowData) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::new();

        let row_box = gtk::Box::new(Orientation::Vertical, 4);
        row_box.set_margin_top(8);
        row_box.set_margin_bottom(8);
        row_box.set_margin_start(8);
        row_box.set_margin_end(8);

        let top = gtk::Box::new(Orientation::Horizontal, 8);
        let graph = gtk::DrawingArea::new();
        graph.set_content_width((g.lane_count as i32) * 12);
        graph.set_content_height(24);
        let gd = g.clone();
        graph.set_draw_func(move |_, cr: &cairo::Context, _w, h| {
            let h = h as f64;
            let lane_width = 12.0;
            let center = |lane: usize| lane_width / 2.0 + lane as f64 * lane_width;
            cr.set_source_rgb(0.5, 0.5, 0.5);
            cr.set_line_width(2.0);

            for (lane, oid_opt) in gd.active_before.iter().enumerate() {
                if oid_opt.is_some() {
                    let x = center(lane);
                    cr.move_to(x, 0.0);
                    cr.line_to(x, h);
                    cr.stroke().ok();
                }
            }

            for &p_lane in &gd.parent_lanes {
                let x0 = center(gd.node_lane);
                let y0 = h / 2.0;
                let x1 = center(p_lane);
                let y1 = h;
                cr.move_to(x0, y0);
                if p_lane == gd.node_lane {
                    cr.line_to(x1, y1);
                } else {
                    cr.curve_to(x0, (y0 + y1) / 2.0, x1, (y0 + y1) / 2.0, x1, y1);
                }
                cr.stroke().ok();
            }

            let x = center(gd.node_lane);
            cr.arc(x, h / 2.0, 3.0, 0.0, std::f64::consts::PI * 2.0);
            cr.fill().ok();
        });

        let hash = gtk::Label::new(Some(&format!("{}", c.short_id)));
        hash.add_css_class("monospace");
        hash.set_xalign(0.0);

        let summary = gtk::Label::new(Some(&c.summary));
        summary.set_xalign(0.0);
        summary.set_ellipsize(pango::EllipsizeMode::End);
        summary.set_hexpand(true);
        let lowered = c.summary.to_lowercase();
        if lowered.contains("crash") {
            summary.add_css_class("commit-warning");
        }

        top.append(&graph);
        top.append(&hash);
        top.append(&summary);

        let tag_box = gtk::Box::new(Orientation::Horizontal, 4);
        for r in &c.refs {
            let tag = gtk::Label::new(Some(r));
            tag.add_css_class("tag-label");
            tag_box.append(&tag);
        }
        top.append(&tag_box);

        let bottom = gtk::Label::new(Some(&format!(
            "{} <{}> • {} • {} parents",
            c.author,
            c.email,
            c.time,
            c.parents.len()
        )));
        bottom.add_css_class("dim-label");
        bottom.set_xalign(0.0);

        row_box.append(&top);
        row_box.append(&bottom);

        row.set_child(Some(&row_box));
        row.set_widget_name(&c.oid);

        row
    }

    pub fn load(&self, commits: Vec<CommitInfo>) {
        {
            let mut all = self.commits.borrow_mut();
            all.clear();
            all.extend(commits.clone());
        }
        self.reload_list(&commits);
    }

    pub fn append(&self, commits: Vec<CommitInfo>) {
        {
            let mut all = self.commits.borrow_mut();
            all.extend(commits.clone());
        }
        let all = self.commits.borrow();
        self.reload_list(&all);
    }

    pub fn connect_on_select<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }

    pub fn filter(&self, query: &str) {
        let all = self.commits.borrow();
        if query.is_empty() {
            self.reload_list(&all);
        } else {
            let q = query.to_lowercase();
            let filtered: Vec<CommitInfo> = all
                .iter()
                .filter(|c| {
                    c.summary.to_lowercase().contains(&q)
                        || c.author.to_lowercase().contains(&q)
                        || c.short_id.contains(&q)
                        || c.oid.contains(&q)
                })
                .cloned()
                .collect();
            self.reload_list(&filtered);
        }
    }

    fn reload_list(&self, commits: &[CommitInfo]) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let graphs = Self::compute_graph(commits);
        for (c, g) in commits.iter().zip(graphs.iter()) {
            let row = Self::row_from_commit(c, g);
            self.list.append(&row);
        }
    }

    fn compute_graph(commits: &[CommitInfo]) -> Vec<GraphRowData> {
        let mut lanes: Vec<Option<String>> = Vec::new();
        let mut rows = Vec::new();

        for c in commits {
            let active_before = lanes.clone();

            let node_lane = if let Some(idx) = lanes.iter().position(|o| o.as_ref() == Some(&c.oid)) {
                idx
            } else {
                let idx = lanes.iter().position(|o| o.is_none()).unwrap_or_else(|| {
                    lanes.push(None);
                    lanes.len() - 1
                });
                idx
            };

            if node_lane >= lanes.len() {
                lanes.resize(node_lane + 1, None);
            }

            let mut parent_lanes = Vec::new();
            for (i, p) in c.parents.iter().enumerate() {
                if i == 0 {
                    parent_lanes.push(node_lane);
                    lanes[node_lane] = Some(p.clone());
                } else {
                    let idx = if let Some(pos) = lanes.iter().position(|o| o.as_ref() == Some(p)) {
                        pos
                    } else {
                        let pos = lanes.iter().position(|o| o.is_none()).unwrap_or_else(|| {
                            lanes.push(None);
                            lanes.len() - 1
                        });
                        pos
                    };
                    parent_lanes.push(idx);
                    if idx >= lanes.len() {
                        lanes.resize(idx + 1, None);
                    }
                    lanes[idx] = Some(p.clone());
                }
            }

            for l in lanes.iter_mut() {
                if l.as_ref() == Some(&c.oid) {
                    *l = None;
                }
            }

            let lane_count = lanes.len();
            rows.push(GraphRowData {
                active_before,
                node_lane,
                parent_lanes,
                lane_count,
            });
        }

        rows
    }
}
