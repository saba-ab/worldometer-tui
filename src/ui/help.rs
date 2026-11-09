//! `?` help popup listing every keybinding.

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::Theme;

pub const KEYBINDINGS: &[(&str, &str, &str)] = &[
    ("1..5  Tab  Shift-Tab", "global", "switch tab"),
    ("j/k  ↓/↑", "lists", "move selection"),
    ("g / G", "lists", "first / last"),
    ("PgDn / PgUp", "lists", "page"),
    ("h/l  ←/→", "tables", "change sort column"),
    ("s", "tables", "toggle asc/desc"),
    ("/", "tables", "search (Enter keep, Esc clear)"),
    ("Enter", "countries", "open country detail"),
    ("Esc / Backspace", "detail, popup", "back"),
    ("r", "global", "force refresh current screen"),
    ("?", "global", "this help"),
    ("q  Ctrl-C", "global", "quit"),
];

pub fn draw(frame: &mut Frame, area: Rect, theme: &Theme) {
    let height = KEYBINDINGS.len() as u16 + 6;
    let [popup] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::horizontal([Constraint::Length(74)])
        .flex(Flex::Center)
        .areas(popup);

    let key_style = theme.fg(theme.accent).add_modifier(Modifier::BOLD);
    let muted = theme.fg(theme.muted);
    let mut lines: Vec<Line> = KEYBINDINGS
        .iter()
        .map(|(keys, scope, action)| {
            Line::from(vec![
                Span::styled(format!(" {keys:<22}"), key_style),
                Span::styled(format!("{scope:<15}"), muted),
                Span::raw(*action),
            ])
        })
        .collect();
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        " Data: worldometers.info, personal use. Cached ≥ 6h.",
        muted,
    ));
    lines.push(Line::styled(" Esc to close", muted));

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title(" Keybindings ")
                .border_style(key_style),
        ),
        popup,
    );
}
