//! Multi-select by song path (click / Shift-range / Ctrl-toggle).

#[derive(Debug, Default, Clone)]
pub struct PathSelection {
    paths: Vec<String>,
    /// Anchor for Shift-range selection.
    anchor: Option<String>,
}

impl PathSelection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.anchor = None;
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    pub fn contains(&self, path: &str) -> bool {
        self.paths.iter().any(|p| p == path)
    }

    pub fn first(&self) -> Option<&str> {
        self.paths.first().map(|s| s.as_str())
    }

    /// Plain click: select only this path.
    pub fn select_only(&mut self, path: String) {
        self.paths.clear();
        self.paths.push(path.clone());
        self.anchor = Some(path);
    }

    /// Ctrl/Cmd click: toggle membership.
    pub fn toggle(&mut self, path: String) {
        if let Some(i) = self.paths.iter().position(|p| p == &path) {
            self.paths.remove(i);
            if self.anchor.as_deref() == Some(path.as_str()) {
                self.anchor = self.paths.last().cloned();
            }
        } else {
            self.paths.push(path.clone());
            self.anchor = Some(path);
        }
    }

    /// Shift click: select contiguous range in `ordered` from anchor to `path`.
    pub fn select_range<S: AsRef<str>>(&mut self, ordered: &[S], path: String) {
        let Some(anchor) = self.anchor.clone() else {
            self.select_only(path);
            return;
        };
        let Some(a) = ordered.iter().position(|p| p.as_ref() == anchor) else {
            self.select_only(path);
            return;
        };
        let Some(b) = ordered.iter().position(|p| p.as_ref() == path) else {
            self.select_only(path);
            return;
        };
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.paths = ordered[lo..=hi].iter().map(|s| s.as_ref().to_string()).collect();
        // Keep original anchor so further Shift-clicks extend from it.
        self.anchor = Some(anchor);
    }

    /// Apply modifiers: shift = range, ctrl = toggle, else only.
    pub fn click<S: AsRef<str>>(&mut self, ordered: &[S], path: String, shift: bool, ctrl: bool) {
        if shift {
            self.select_range(ordered, path);
        } else if ctrl {
            self.toggle(path);
        } else {
            self.select_only(path);
        }
    }

    /// Collect songs from `all` whose paths are selected, preserving selection order.
    pub fn collect_songs(&self, all: &[mu_core::Song]) -> Vec<mu_core::Song> {
        self.paths
            .iter()
            .filter_map(|p| all.iter().find(|s| s.path == *p).cloned())
            .collect()
    }
}
