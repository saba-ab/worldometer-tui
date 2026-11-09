//! Table screens (Countries, GDP, CO₂, Area).

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell as UiCell, HighlightSpacing, Paragraph, Row, Table, TableState};

use super::{Theme, draw_placeholder, format};
use crate::app::TableTab;
use crate::tables::{Cell, ColKind, ColumnDef};

const HIGHLIGHT: &str = "› ";
const SPACING: u16 = 1;

/// Formats a cell for display according to its column kind.
pub fn format_cell(cell: &Cell, kind: ColKind) -> String {
    match cell {
        Cell::Text(text) => text.clone(),
        Cell::Num(None) => format::MISSING.to_string(),
        Cell::Num(Some(v)) => match kind {
            ColKind::Text | ColKind::Int => format::int(*v),
            ColKind::Float(decimals) => format::float(*v, decimals as usize),
            ColKind::SignedPct => format::signed_pct(*v),
            ColKind::Pct => format::pct(*v),
            ColKind::Usd => format::usd(*v),
            ColKind::UsdCompact => format::usd_compact(*v),
        },
    }
}

/// Style for a value: signed percentages are green/red.
pub fn cell_style(cell: &Cell, kind: ColKind, theme: &Theme) -> Style {
    match (cell, kind) {
        (Cell::Num(Some(v)), ColKind::SignedPct) if *v > 0.0 => theme.fg(theme.positive),
        (Cell::Num(Some(v)), ColKind::SignedPct) if *v < 0.0 => theme.fg(theme.negative),
        (Cell::Num(None), _) => theme.fg(theme.muted),
        _ => Style::default(),
    }
}

/// Leading columns that fit in `width` (always at least the first two).
pub fn visible_columns(columns: &[ColumnDef], width: u16) -> usize {
    let available = width.saturating_sub(HIGHLIGHT.chars().count() as u16);
    let mut used = 0u16;
    let mut count = 0;
    for (i, column) in columns.iter().enumerate() {
        let needed = column.width + if i > 0 { SPACING } else { 0 };
        if used + needed > available && count >= 2 {
            break;
        }
        used += needed;
        count += 1;
    }
    count
}

pub fn aligned(text: String, kind: ColKind, style: Style) -> UiCell<'static> {
    let alignment = if kind.is_numeric() {
        Alignment::Right
    } else {
        Alignment::Left
    };
    UiCell::from(Line::styled(text, style).alignment(alignment))
}

/// Draws a table tab and returns the number of data rows per page.
pub fn draw(
    frame: &mut Frame,
    area: Rect,
    tab: &mut TableTab,
    theme: &Theme,
    searching: bool,
) -> usize {
    let Some(table) = tab.slot.data.as_ref() else {
        draw_placeholder(frame, area, &tab.slot, tab.kind.title(), theme);
        return 1;
    };
    let view = &mut tab.view;

    let mut area = area;
    if searching || !view.query.is_empty() {
        let [search_area, rest] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
        let cursor = if searching { "▏" } else { "" };
        let line = Line::from(vec![
            Span::styled(
                " Search: ",
                theme.fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}{cursor}", view.query)),
            Span::styled(
                format!("   {} of {} rows", view.order.len(), table.rows.len()),
                theme.fg(theme.muted),
            ),
        ]);
        frame.render_widget(line, search_area);
        area = rest;
    }

    let columns = &table.columns[..visible_columns(table.columns, area.width)];
    let arrow = if view.descending { " ▼" } else { " ▲" };
    let header = Row::new(columns.iter().enumerate().map(|(i, column)| {
        let title = if i == view.sort_col {
            format!("{}{arrow}", column.title)
        } else {
            column.title.to_string()
        };
        aligned(title, column.kind, Style::default())
    }))
    .style(theme.fg(theme.accent).add_modifier(Modifier::BOLD));

    if view.order.is_empty() {
        let message = Paragraph::new(Line::styled("  No matching rows", theme.fg(theme.muted)));
        let [header_area, message_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        frame.render_widget(
            Table::new(Vec::<Row>::new(), widths(columns)).header(header),
            header_area,
        );
        frame.render_widget(message, message_area);
        return 1;
    }

    let rows = view.order.iter().map(|&index| {
        let row = &table.rows[index];
        Row::new(columns.iter().zip(&row.cells).map(|(column, cell)| {
            aligned(
                format_cell(cell, column.kind),
                column.kind,
                cell_style(cell, column.kind, theme),
            )
        }))
    });

    let widget = Table::new(rows, widths(columns))
        .header(header)
        .column_spacing(SPACING)
        .row_highlight_style(theme.selected())
        .highlight_symbol(HIGHLIGHT)
        .highlight_spacing(HighlightSpacing::Always);
    let mut state = TableState::default()
        .with_offset(view.offset)
        .with_selected(Some(view.selected));
    frame.render_stateful_widget(widget, area, &mut state);
    view.offset = state.offset();

    area.height.saturating_sub(1).max(1) as usize
}

fn widths(columns: &[ColumnDef]) -> Vec<Constraint> {
    columns
        .iter()
        .map(|c| Constraint::Length(c.width))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CountryRow;
    use crate::tables::TableRow;

    #[test]
    fn formats_cells_by_kind() {
        assert_eq!(
            format_cell(&Cell::Num(Some(1_476_625_576.0)), ColKind::Int),
            "1,476,625,576"
        );
        assert_eq!(
            format_cell(&Cell::Num(Some(-0.23)), ColKind::SignedPct),
            "−0.23 %"
        );
        assert_eq!(format_cell(&Cell::Num(None), ColKind::Pct), "—");
        assert_eq!(
            format_cell(&Cell::Text("India".into()), ColKind::Text),
            "India"
        );
    }

    #[test]
    fn columns_are_dropped_from_the_right_when_narrow() {
        let columns = CountryRow::COLUMNS;
        assert_eq!(visible_columns(columns, 500), columns.len());
        assert_eq!(visible_columns(columns, 10), 2);
        let n = visible_columns(columns, 120);
        assert!(n > 5 && n < columns.len());
    }
}
