//! Live counters dashboard: cards grouped by category, recomputed every frame
//! from the clock (no I/O).

use std::time::SystemTime;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::{Theme, format};
use crate::app::App;
use crate::counters::{Category, Counter, energy_counters};

const CARD_HEIGHT: u16 = 3;

fn columns_for(width: u16) -> u16 {
    match width {
        w if w >= 120 => 3,
        w if w >= 80 => 2,
        _ => 1,
    }
}

/// One section of the dashboard.
enum Section<'a> {
    Cards {
        title: String,
        note: Option<String>,
        counters: Vec<&'a Counter>,
    },
    Message {
        title: String,
        text: String,
        warn: bool,
    },
}

impl Section<'_> {
    fn height(&self, columns: u16) -> u16 {
        match self {
            Section::Cards { note, counters, .. } => {
                let rows = (counters.len() as u16).div_ceil(columns);
                1 + u16::from(note.is_some()) + rows * CARD_HEIGHT
            }
            Section::Message { .. } => 2,
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, app: &App, now: SystemTime) {
    let theme = &app.theme;
    let energy: Vec<Counter> = app
        .energy
        .data
        .as_ref()
        .map(energy_counters)
        .unwrap_or_default();
    let mut sections: Vec<Section> = Vec::new();

    match &app.estimated {
        Ok(counters) => {
            let mut categories: Vec<Category> = counters.iter().map(|c| c.category).collect();
            categories.sort();
            categories.dedup();
            for category in categories {
                let group: Vec<&Counter> =
                    counters.iter().filter(|c| c.category == category).collect();
                sections.push(Section::Cards {
                    title: category.label().to_string(),
                    note: None,
                    counters: group,
                });
            }
        }
        Err(error) => sections.push(Section::Message {
            title: "Population counters".into(),
            text: format!("unavailable: {error}"),
            warn: true,
        }),
    }

    if !energy.is_empty() {
        sections.push(Section::Cards {
            title: "Energy — worldometers.info".into(),
            note: Some(
                "Constants from worldometers' live-counter script; values computed locally each second."
                    .into(),
            ),
            counters: energy.iter().collect(),
        });
    } else if let Some(error) = &app.energy.error {
        sections.push(Section::Message {
            title: "Energy".into(),
            text: format!("unavailable: {error} — press r to retry"),
            warn: true,
        });
    } else {
        sections.push(Section::Message {
            title: "Energy".into(),
            text: "loading constants from worldometers.info…".into(),
            warn: false,
        });
    }

    let columns = columns_for(area.width);
    let mut y = area.y;
    for section in &sections {
        let height = section.height(columns).min(area.bottom().saturating_sub(y));
        if height == 0 {
            break;
        }
        let section_area = Rect::new(area.x, y, area.width, height);
        draw_section(frame, section_area, section, columns, app, now, theme);
        y += height;
    }
}

fn draw_section(
    frame: &mut Frame,
    area: Rect,
    section: &Section,
    columns: u16,
    app: &App,
    now: SystemTime,
    theme: &Theme,
) {
    let title_style = theme.fg(theme.accent).add_modifier(Modifier::BOLD);
    match section {
        Section::Message { title, text, warn } => {
            let style = if *warn {
                theme.fg(theme.warn)
            } else {
                theme.fg(theme.muted)
            };
            frame.render_widget(
                Paragraph::new(vec![
                    Line::styled(format!(" {title}"), title_style),
                    Line::styled(format!(" {text}"), style),
                ]),
                area,
            );
        }
        Section::Cards {
            title,
            note,
            counters,
        } => {
            let [title_area, note_area, grid] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(u16::from(note.is_some())),
                Constraint::Min(0),
            ])
            .areas(area);
            frame.render_widget(Line::styled(format!(" {title}"), title_style), title_area);
            if let Some(note) = note {
                frame.render_widget(
                    Line::styled(format!(" {note}"), theme.fg(theme.muted)),
                    note_area,
                );
            }

            let card_width = grid.width / columns;
            for (i, counter) in counters.iter().enumerate() {
                let col = i as u16 % columns;
                let row = i as u16 / columns;
                let y = grid.y + row * CARD_HEIGHT;
                if y + CARD_HEIGHT > grid.bottom() {
                    break;
                }
                let card = Rect::new(grid.x + col * card_width, y, card_width, CARD_HEIGHT);
                draw_card(frame, card, counter, app, now, theme);
            }
        }
    }
}

fn draw_card(
    frame: &mut Frame,
    area: Rect,
    counter: &Counter,
    app: &App,
    now: SystemTime,
    theme: &Theme,
) {
    let block = Block::bordered()
        .border_style(theme.fg(theme.muted))
        .title(Span::raw(format!(" {} ", counter.label)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let value = counter.value_at(now, app.tz);
    let mut value_spans = vec![Span::styled(
        format!(" {}", format::int(value)),
        theme.fg(theme.accent).add_modifier(Modifier::BOLD),
    )];
    if let Some(unit) = &counter.unit {
        value_spans.push(Span::styled(format!(" {unit}"), theme.fg(theme.muted)));
    }
    frame.render_widget(Line::from(value_spans), inner);
}
