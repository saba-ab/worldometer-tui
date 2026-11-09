//! `App`: the single owner of UI state. Reacts to keys ([`App::on_key`]) and
//! worker messages ([`App::on_msg`]). Never does I/O: page loads are queued as
//! [`Request`]s that the main loop hands to the worker.

use chrono::{FixedOffset, TimeDelta};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

use crate::counters::Counter;
use crate::model::{CountryDetail, EnergyConstants};
use crate::msg::{Freshness, Msg, PageData, PageKey, Request};
use crate::tables::{Table, TableKind, TableView};
use crate::ui::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tab {
    #[default]
    Countries,
    Live,
    Gdp,
    Co2,
    Area,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Countries, Tab::Live, Tab::Gdp, Tab::Co2, Tab::Area];

    pub fn title(self) -> &'static str {
        match self.table_kind() {
            Some(kind) => kind.title(),
            None => "Live",
        }
    }

    pub fn table_kind(self) -> Option<TableKind> {
        match self {
            Tab::Countries => Some(TableKind::Population),
            Tab::Live => None,
            Tab::Gdp => Some(TableKind::Gdp),
            Tab::Co2 => Some(TableKind::Co2),
            Tab::Area => Some(TableKind::Area),
        }
    }

    fn index(self) -> usize {
        Tab::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }

    fn cycle(self, delta: isize) -> Tab {
        let n = Tab::ALL.len() as isize;
        Tab::ALL[(self.index() as isize + delta).rem_euclid(n) as usize]
    }
}

/// Load state of one piece of data. Data is kept across failed refreshes.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot<T> {
    pub data: Option<T>,
    pub loading: bool,
    pub error: Option<String>,
    pub freshness: Option<Freshness>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            data: None,
            loading: false,
            error: None,
            freshness: None,
        }
    }
}

impl<T> Slot<T> {
    fn needs_initial_load(&self) -> bool {
        self.data.is_none() && !self.loading && self.error.is_none()
    }

    fn loaded(&mut self, data: T, freshness: Freshness) {
        self.data = Some(data);
        self.loading = false;
        self.error = None;
        self.freshness = Some(freshness);
    }

    fn failed(&mut self, error: String) {
        self.loading = false;
        self.error = Some(error);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableTab {
    pub kind: TableKind,
    pub slot: Slot<Table>,
    pub view: TableView,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetailState {
    pub url: String,
    /// Name from the table, shown while loading.
    pub name: String,
    pub slot: Slot<CountryDetail>,
    /// Selected row of the history table.
    pub selected: usize,
    pub offset: usize,
}

pub struct App {
    pub tab: Tab,
    pub tables: Vec<TableTab>,
    pub energy: Slot<EnergyConstants>,
    /// Option B counters, or why they could not be loaded.
    pub estimated: Result<Vec<Counter>, String>,
    pub detail: Option<DetailState>,
    pub show_help: bool,
    pub searching: bool,
    pub should_quit: bool,
    /// One-line message for the status bar, cleared by the next key.
    pub notice: Option<String>,
    /// Rows per page, maintained by the UI for PgUp/PgDn.
    pub page_size: usize,
    pub ttl: TimeDelta,
    /// Calendar used for daily/yearly counter resets.
    pub tz: FixedOffset,
    pub theme: Theme,
    outbox: Vec<Request>,
}

impl App {
    pub fn new(
        start: Tab,
        estimated: Result<Vec<Counter>, String>,
        ttl: TimeDelta,
        tz: FixedOffset,
        theme: Theme,
    ) -> Self {
        let mut app = Self {
            tab: start,
            tables: TableKind::ALL
                .into_iter()
                .map(|kind| TableTab {
                    kind,
                    slot: Slot::default(),
                    view: TableView::default(),
                })
                .collect(),
            energy: Slot::default(),
            estimated,
            detail: None,
            show_help: false,
            searching: false,
            should_quit: false,
            notice: None,
            page_size: 20,
            ttl,
            tz,
            theme,
            outbox: Vec::new(),
        };
        app.switch_tab(start);
        app
    }

    /// Requests queued since the last call, for the main loop to send.
    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.outbox)
    }

    pub fn table(&self, kind: TableKind) -> &TableTab {
        self.tables
            .iter()
            .find(|t| t.kind == kind)
            .expect("every TableKind has a tab")
    }

    fn table_mut(&mut self, kind: TableKind) -> &mut TableTab {
        self.tables
            .iter_mut()
            .find(|t| t.kind == kind)
            .expect("every TableKind has a tab")
    }

    /// The country detail, if it is the screen being shown.
    pub fn visible_detail(&self) -> Option<&DetailState> {
        self.detail.as_ref().filter(|_| self.tab == Tab::Countries)
    }

    fn request(&mut self, key: PageKey, force: bool) {
        if !self.outbox.iter().any(|r| r.key == key) {
            self.outbox.push(Request { key, force });
        }
    }

    pub fn switch_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.searching = false;
        match tab.table_kind() {
            Some(kind) => {
                let slot = &mut self.table_mut(kind).slot;
                if slot.needs_initial_load() {
                    slot.loading = true;
                    self.request(kind.page_key(), false);
                }
            }
            None => {
                if self.energy.needs_initial_load() {
                    self.energy.loading = true;
                    self.request(PageKey::Counters, false);
                }
            }
        }
    }

    /// `r`: force a network fetch for the current screen.
    pub fn refresh(&mut self) {
        if let Some(detail) = self.detail.as_mut().filter(|_| self.tab == Tab::Countries) {
            detail.slot.loading = true;
            let key = PageKey::Country(detail.url.clone());
            self.request(key, true);
            return;
        }
        match self.tab.table_kind() {
            Some(kind) => {
                self.table_mut(kind).slot.loading = true;
                self.request(kind.page_key(), true);
            }
            None => {
                self.energy.loading = true;
                self.request(PageKey::Counters, true);
            }
        }
    }

    fn open_detail(&mut self) {
        let tab = self.table(TableKind::Population);
        let Some(table) = tab.slot.data.as_ref() else {
            return;
        };
        let Some(row) = tab.view.selected_row(table) else {
            return;
        };
        let name = row
            .cells
            .get(crate::tables::NAME_COL)
            .and_then(|c| c.text())
            .unwrap_or("?")
            .to_string();
        let Some(url) = row.detail_url.clone() else {
            self.notice = Some(format!("No detail page for {name}"));
            return;
        };
        self.detail = Some(DetailState {
            url: url.clone(),
            name,
            slot: Slot {
                loading: true,
                ..Slot::default()
            },
            selected: 0,
            offset: 0,
        });
        self.request(PageKey::Country(url), false);
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        self.notice = None;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        if self.show_help {
            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('?') | KeyCode::Enter => {
                    self.show_help = false
                }
                _ => {}
            }
            return;
        }
        if self.searching {
            self.on_search_key(key);
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Tab => self.switch_tab(self.tab.cycle(1)),
            KeyCode::BackTab => self.switch_tab(self.tab.cycle(-1)),
            KeyCode::Char(c @ '1'..='9') => {
                let index = c as usize - '1' as usize;
                if let Some(tab) = Tab::ALL.get(index) {
                    self.switch_tab(*tab);
                }
            }
            _ if self.visible_detail().is_some() => self.on_detail_key(key),
            _ => self.on_table_key(key),
        }
    }

    fn on_search_key(&mut self, key: KeyEvent) {
        let Some(kind) = self.tab.table_kind() else {
            self.searching = false;
            return;
        };
        let tab = self.table_mut(kind);
        let Some(table) = tab.slot.data.as_ref() else {
            self.searching = false;
            return;
        };
        match key.code {
            KeyCode::Esc => {
                tab.view.set_query(String::new(), table);
                self.searching = false;
            }
            KeyCode::Enter => self.searching = false,
            KeyCode::Backspace => {
                let mut query = tab.view.query.clone();
                query.pop();
                tab.view.set_query(query, table);
            }
            KeyCode::Down => tab.view.move_by(1),
            KeyCode::Up => tab.view.move_by(-1),
            KeyCode::Char(c) => {
                let query = format!("{}{c}", tab.view.query);
                tab.view.set_query(query, table);
            }
            _ => {}
        }
    }

    fn on_detail_key(&mut self, key: KeyEvent) {
        let page = self.page_size.max(1) as isize;
        let Some(detail) = self.detail.as_mut() else {
            return;
        };
        let rows = detail
            .slot
            .data
            .as_ref()
            .map_or(0, |d| d.history.len() + d.forecast.len());
        let last = rows.saturating_sub(1) as isize;
        let current = detail.selected as isize;
        let target = match key.code {
            KeyCode::Esc | KeyCode::Backspace => {
                self.detail = None;
                return;
            }
            KeyCode::Char('j') | KeyCode::Down => current + 1,
            KeyCode::Char('k') | KeyCode::Up => current - 1,
            KeyCode::Char('g') | KeyCode::Home => 0,
            KeyCode::Char('G') | KeyCode::End => last,
            KeyCode::PageDown => current + page,
            KeyCode::PageUp => current - page,
            _ => return,
        };
        detail.selected = target.clamp(0, last.max(0)) as usize;
    }

    fn on_table_key(&mut self, key: KeyEvent) {
        let Some(kind) = self.tab.table_kind() else {
            return;
        };
        let page = self.page_size.max(1) as isize;
        match key.code {
            KeyCode::Enter => {
                if kind == TableKind::Population {
                    self.open_detail();
                }
                return;
            }
            KeyCode::Char('/') => {
                self.searching = self.table(kind).slot.data.is_some();
                return;
            }
            _ => {}
        }
        let tab = self.table_mut(kind);
        let Some(table) = tab.slot.data.as_ref() else {
            return;
        };
        let view = &mut tab.view;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => view.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => view.move_by(-1),
            KeyCode::Char('g') | KeyCode::Home => view.first(),
            KeyCode::Char('G') | KeyCode::End => view.last(),
            KeyCode::PageDown => view.move_by(page),
            KeyCode::PageUp => view.move_by(-page),
            KeyCode::Char('l') | KeyCode::Right => view.shift_sort(1, table),
            KeyCode::Char('h') | KeyCode::Left => view.shift_sort(-1, table),
            KeyCode::Char('s') => view.toggle_direction(table),
            KeyCode::Esc if !view.query.is_empty() => view.set_query(String::new(), table),
            _ => {}
        }
    }

    pub fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Loaded {
                key,
                data,
                freshness,
            } => match (key, data) {
                (PageKey::Country(url), PageData::Country(detail)) => {
                    if let Some(state) = self.detail.as_mut().filter(|d| d.url == url) {
                        // Select the newest historical year (below the forecast rows).
                        state.selected = detail.forecast.len();
                        state.slot.loaded(detail, freshness);
                    }
                }
                (PageKey::Counters, PageData::Energy(constants)) => {
                    self.energy.loaded(constants, freshness)
                }
                (key, PageData::Table(table)) => {
                    if let Some(tab) = self.tables.iter_mut().find(|t| t.kind.page_key() == key) {
                        tab.view.rebuild(&table);
                        tab.slot.loaded(table, freshness);
                    }
                }
                _ => {}
            },
            Msg::Failed { key, error } => match key {
                PageKey::Country(url) => {
                    if let Some(state) = self.detail.as_mut().filter(|d| d.url == url) {
                        state.slot.failed(error);
                    }
                }
                PageKey::Counters => self.energy.failed(error),
                key => {
                    if let Some(tab) = self.tables.iter_mut().find(|t| t.kind.page_key() == key) {
                        tab.slot.failed(error);
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CountryRow;
    use chrono::{TimeZone, Utc};
    use ratatui::crossterm::event::KeyEvent;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app() -> App {
        App::new(
            Tab::Countries,
            Ok(Vec::new()),
            TimeDelta::hours(6),
            FixedOffset::east_opt(0).unwrap(),
            Theme::default(),
        )
    }

    fn loaded_population(app: &mut App) {
        let rows = vec![CountryRow {
            rank: 1,
            name: "Georgia".into(),
            detail_url: Some("https://x/georgia-population/".into()),
            population: 3_804_642,
            yearly_change_pct: None,
            net_change: None,
            density_per_km2: None,
            land_area_km2: None,
            migrants_net: None,
            fertility_rate: None,
            median_age: None,
            urban_pop_pct: None,
            world_share_pct: None,
        }];
        app.on_msg(Msg::Loaded {
            key: PageKey::Population,
            data: PageData::Table(Table::from_rows(&rows)),
            freshness: Freshness::Fresh {
                fetched_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            },
        });
    }

    #[test]
    fn start_requests_the_default_tab_once() {
        let mut app = app();
        assert_eq!(
            app.take_requests(),
            vec![Request {
                key: PageKey::Population,
                force: false
            }]
        );
        app.switch_tab(Tab::Countries);
        assert!(app.take_requests().is_empty());
    }

    #[test]
    fn switching_to_live_loads_counters_lazily() {
        let mut app = app();
        app.take_requests();
        app.on_key(key(KeyCode::Char('2')));
        assert_eq!(app.tab, Tab::Live);
        assert_eq!(app.take_requests()[0].key, PageKey::Counters);
    }

    #[test]
    fn refresh_forces_network() {
        let mut app = app();
        app.take_requests();
        app.on_key(key(KeyCode::Char('r')));
        assert_eq!(
            app.take_requests(),
            vec![Request {
                key: PageKey::Population,
                force: true
            }]
        );
    }

    #[test]
    fn enter_opens_detail_and_esc_returns() {
        let mut app = app();
        loaded_population(&mut app);
        app.take_requests();
        app.on_key(key(KeyCode::Enter));
        let detail = app.visible_detail().unwrap();
        assert_eq!(detail.name, "Georgia");
        assert_eq!(
            app.take_requests()[0].key,
            PageKey::Country("https://x/georgia-population/".into())
        );
        app.on_key(key(KeyCode::Esc));
        assert!(app.detail.is_none());
    }

    #[test]
    fn search_mode_edits_query() {
        let mut app = app();
        loaded_population(&mut app);
        app.on_key(key(KeyCode::Char('/')));
        assert!(app.searching);
        for c in "xyz".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        assert_eq!(app.table(TableKind::Population).view.order.len(), 0);
        // 'q' is text while searching, not quit.
        app.on_key(key(KeyCode::Char('q')));
        assert!(!app.should_quit);
        app.on_key(key(KeyCode::Esc));
        assert!(!app.searching);
        assert_eq!(app.table(TableKind::Population).view.query, "");
        assert_eq!(app.table(TableKind::Population).view.order.len(), 1);
    }

    #[test]
    fn failure_keeps_existing_data() {
        let mut app = app();
        loaded_population(&mut app);
        app.on_msg(Msg::Failed {
            key: PageKey::Population,
            error: "boom".into(),
        });
        let slot = &app.table(TableKind::Population).slot;
        assert!(slot.data.is_some());
        assert_eq!(slot.error.as_deref(), Some("boom"));
    }

    #[test]
    fn quit_keys() {
        let mut app = app();
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }
}
