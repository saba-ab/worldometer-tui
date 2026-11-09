//! Drawing. `draw` is a pure function of the app state and the clock passed in
//! (so tests can freeze time); the only state it writes back is scroll offsets
//! and the page size, which depend on the terminal size.

pub mod dashboard;
pub mod detail;
pub mod format;
pub mod help;
pub mod table;

use std::time::SystemTime;

use chrono::{DateTime, TimeDelta, Utc};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::{App, Slot, Tab};
use crate::config::ThemeName;
use crate::msg::Freshness;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub accent: Color,
    pub positive: Color,
    pub negative: Color,
    pub muted: Color,
    pub warn: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Color::Cyan,
            positive: Color::Green,
            negative: Color::Red,
            muted: Color::DarkGray,
            warn: Color::Yellow,
        }
    }
}

impl Theme {
    pub fn mono() -> Self {
        Self {
            accent: Color::Reset,
            positive: Color::Reset,
            negative: Color::Reset,
            muted: Color::Reset,
            warn: Color::Reset,
        }
    }

    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Default => Self::default(),
            ThemeName::Mono => Self::mono(),
        }
    }

    pub fn fg(&self, color: Color) -> Style {
        Style::default().fg(color)
    }

    pub fn selected(&self) -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
    }
}

pub fn draw(frame: &mut Frame, app: &mut App, now: SystemTime) {
    let [tabs_area, body, status_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_tabs(frame, tabs_area, app);

    let theme = app.theme;
    let searching = app.searching;
    if app.tab == Tab::Countries && app.detail.is_some() {
        if let Some(detail) = app.detail.as_mut() {
            app.page_size = detail::draw(frame, body, detail, &theme);
        }
    } else if let Some(kind) = app.tab.table_kind() {
        let tab = app
            .tables
            .iter_mut()
            .find(|t| t.kind == kind)
            .expect("every TableKind has a tab");
        app.page_size = table::draw(frame, body, tab, &theme, searching);
    } else {
        dashboard::draw(frame, body, app, now);
    }

    draw_status(frame, status_area, app, now);

    if app.show_help {
        help::draw(frame, frame.area(), &theme);
    }
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let mut spans = vec![Span::styled(
        " worldo-tui ",
        theme.fg(theme.accent).add_modifier(Modifier::BOLD),
    )];
    for (i, tab) in Tab::ALL.iter().enumerate() {
        let label = format!(" [{}] {} ", i + 1, tab.title());
        let style = if *tab == app.tab {
            theme.selected().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label, style));
    }
    frame.render_widget(Line::from(spans), area);
}

/// Status of a slot for the status bar, e.g. `Updated 2h ago` or `OFFLINE · STALE (3d)`.
pub fn status_text<T>(slot: &Slot<T>, now: DateTime<Utc>, ttl: TimeDelta) -> (String, bool) {
    if slot.loading {
        let text = if slot.data.is_some() {
            "Refreshing…"
        } else {
            "Loading…"
        };
        return (text.to_string(), false);
    }
    if slot.error.is_some() && slot.data.is_some() {
        return ("Refresh failed — r to retry".to_string(), true);
    }
    match &slot.freshness {
        Some(freshness) => {
            let age = now.signed_duration_since(freshness.fetched_at());
            let age_text = format::age(age.num_seconds());
            let ago = if age_text == "just now" {
                age_text.clone()
            } else {
                format!("{age_text} ago")
            };
            match freshness {
                Freshness::Fresh { .. } => (format!("Updated {ago}"), false),
                Freshness::Offline { .. } if age >= ttl => {
                    (format!("OFFLINE · STALE ({age_text})"), true)
                }
                Freshness::Offline { .. } => (format!("OFFLINE · cached {ago}"), true),
            }
        }
        None => (String::new(), false),
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App, now: SystemTime) {
    let theme = &app.theme;
    let now_utc = DateTime::<Utc>::from(now);

    let (right, warn) = if let Some(notice) = &app.notice {
        (notice.clone(), true)
    } else if let Some(detail) = app.visible_detail() {
        status_text(&detail.slot, now_utc, app.ttl)
    } else if let Some(kind) = app.tab.table_kind() {
        status_text(&app.table(kind).slot, now_utc, app.ttl)
    } else {
        status_text(&app.energy, now_utc, app.ttl)
    };

    let left: Line = if app.searching {
        let query = app
            .tab
            .table_kind()
            .map(|k| app.table(k).view.query.clone())
            .unwrap_or_default();
        Line::from(vec![
            Span::styled(" /", theme.fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::raw(query),
            Span::styled("▏", theme.fg(theme.accent)),
            Span::styled("  Enter keep · Esc clear", theme.fg(theme.muted)),
        ])
    } else {
        let hints = if app.visible_detail().is_some() {
            "Esc back  j/k scroll  r refresh  ? help  q quit"
        } else if app.tab == Tab::Countries {
            "/ search  h/l sort col  s asc/desc  Enter detail  r refresh  ? help  q quit"
        } else if app.tab.table_kind().is_some() {
            "/ search  h/l sort col  s asc/desc  r refresh  ? help  q quit"
        } else {
            "Tab switch  r refresh  ? help  q quit"
        };
        Line::styled(format!(" {hints}"), theme.fg(theme.muted))
    };

    let right_width = (right.chars().count() as u16 + 1).min(area.width / 2);
    let [left_area, right_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(right_width)]).areas(area);
    frame.render_widget(left, left_area);
    let right_style = if warn {
        theme.fg(theme.warn).add_modifier(Modifier::BOLD)
    } else {
        theme.fg(theme.muted)
    };
    frame.render_widget(
        Paragraph::new(Span::styled(right, right_style)).alignment(Alignment::Right),
        right_area,
    );
}

/// Body shown when a slot has no data yet: loading text or the error.
pub fn draw_placeholder<T>(
    frame: &mut Frame,
    area: Rect,
    slot: &Slot<T>,
    what: &str,
    theme: &Theme,
) {
    let lines = match &slot.error {
        Some(error) => vec![
            Line::styled(
                format!("Could not load {what}"),
                theme.fg(theme.negative).add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::raw(error.clone()),
            Line::raw(""),
            Line::styled("press r to retry", theme.fg(theme.muted)),
        ],
        None => vec![Line::styled(
            format!("Loading {what}…"),
            theme.fg(theme.muted),
        )],
    };
    let inner = area.inner(ratatui::layout::Margin::new(2, 1));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 9, h, 0, 0).unwrap()
    }

    #[test]
    fn status_texts() {
        let ttl = TimeDelta::hours(6);
        let mut slot: Slot<()> = Slot {
            data: Some(()),
            freshness: Some(Freshness::Fresh { fetched_at: at(0) }),
            ..Slot::default()
        };
        assert_eq!(status_text(&slot, at(2), ttl).0, "Updated 2h ago");
        assert_eq!(status_text(&slot, at(0), ttl).0, "Updated just now");

        slot.freshness = Some(Freshness::Offline {
            fetched_at: at(0),
            error: "x".into(),
        });
        assert_eq!(
            status_text(&slot, at(1), ttl),
            ("OFFLINE · cached 1h ago".into(), true)
        );
        assert_eq!(
            status_text(&slot, at(0), ttl),
            ("OFFLINE · cached just now".into(), true)
        );
        let three_days = at(0) + TimeDelta::days(3);
        assert_eq!(
            status_text(&slot, three_days, ttl),
            ("OFFLINE · STALE (3d)".into(), true)
        );

        slot.loading = true;
        assert_eq!(status_text(&slot, at(1), ttl).0, "Refreshing…");
    }
}
