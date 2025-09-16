use chrono::Local;
use humanize_duration::prelude::DurationExt;
use humanize_duration::Truncate;
use num_format::{Locale, ToFormattedString};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Frame,
    style::{Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Padding, Paragraph, Row, Table},
};

use crate::app::App;

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}

pub fn render(app: &mut App, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(1),
            Constraint::Length(9),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded);

    let text = vec![
        Line::from(match &app.metadata.app.title {
            Some(title) => format!(
                "{} ({}): {} jobs running",
                title,
                app.current_profile().name,
                app.status.total
            ),
            None => format!(
                "({}): {} jobs running",
                app.current_profile().name,
                app.status.total
            ),
        }),
        Line::from(
            match (&app.metadata.app.version, &app.metadata.app.ftm_version) {
                (Some(aleph), Some(ftm)) => format!("version: {aleph}, followthemoney: {ftm}"),
                (None, Some(ftm)) => format!("followthemoney: {ftm}"),
                (Some(aleph), None) => format!("version: {aleph}"),
                (None, None) => String::default(),
            },
        ),
    ];
    let title = Paragraph::new(text).block(title_block);
    f.render_widget(title, chunks[0]);

    let mut rows = Vec::new();

    for result in &app.status.results {
        // ROW 1: Collection row
        let collection_id = match &result.collection {
            Some(c) => c.collection_id.clone(),
            None => "-".to_string(),
        };
        let collection_foreign_id = match &result.collection {
            Some(c) => c.foreign_id.clone(),
            None => "-".to_string(),
        };
        let collection_label = match &result.collection {
            Some(c) => c.label.clone(),
            None => result.name.clone(),
        };
        let start_timestamp = result.min_ts.as_ref().unwrap_or(&"-".to_string()).clone();

        rows.push(Row::new(vec![
            collection_id,
            collection_foreign_id,
            collection_label,
            start_timestamp,
            result.todo.to_formatted_string(&Locale::en),
            result.doing.to_formatted_string(&Locale::en),
            result.succeeded.to_formatted_string(&Locale::en),
            result.failed.to_formatted_string(&Locale::en),
            result.aborted.to_formatted_string(&Locale::en),
            result.aborting.to_formatted_string(&Locale::en),
            result.cancelled.to_formatted_string(&Locale::en),
        ]).style(Style::new().add_modifier(Modifier::BOLD)));

        // ROW 2+: Task rows
        for batch in &result.batches {
            for queue in &batch.queues {
                for task in &queue.tasks {
                    let task_start_timestamp = task.min_ts.as_ref().unwrap_or(&"-".to_string()).clone();

                    rows.push(Row::new(vec![
                        "".to_string(), // Empty collection ID column
                        batch.name.clone(), // Batch name in foreign ID column
                        format!("  {}", task.name), // Indented task name in label column
                        task_start_timestamp, // Task timestamp in same column as collection timestamp
                        task.todo.to_formatted_string(&Locale::en),
                        task.doing.to_formatted_string(&Locale::en),
                        task.succeeded.to_formatted_string(&Locale::en),
                        task.failed.to_formatted_string(&Locale::en),
                        task.aborted.to_formatted_string(&Locale::en),
                        task.aborting.to_formatted_string(&Locale::en),
                        task.cancelled.to_formatted_string(&Locale::en),
                    ]));
                }
            }
        }
    }
    let widths = [
        Constraint::Length(15),  // Collection ID
        Constraint::Length(15),  // Foreign ID
        Constraint::Min(20),     // Label
        Constraint::Length(20),  // Start Timestamp
        Constraint::Length(8),   // Todo
        Constraint::Length(8),   // Doing
        Constraint::Length(8),   // Succeeded
        Constraint::Length(8),   // Failed
        Constraint::Length(8),   // Aborted
        Constraint::Length(8),   // Aborting
        Constraint::Length(8),   // Cancelled
    ];
    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                "Collection ID",
                "Foreign ID",
                "Label/Task Name",
                "Start Time",
                "Todo",
                "Doing",
                "Success",
                "Failed",
                "Aborted",
                "Aborting",
                "Cancel",
            ])
            .bottom_margin(1),
        )
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>");

    f.render_stateful_widget(table, chunks[1], &mut app.collection_tablestate);

    if let Some(index) = app.collection_tablestate.selected() {
        // Find which result and row type is selected
        let mut current_row = 0;
        let mut selected_result = None;

        for result in &app.status.results {
            if current_row == index {
                selected_result = Some(result);
                break;
            }
            current_row += 1;

            // Skip task rows for this collection
            for batch in &result.batches {
                for queue in &batch.queues {
                    current_row += queue.tasks.len();
                }
            }
        }

        if let Some(result) = selected_result {
            let title = match &result.collection {
                Some(col) => format!("Collection {} <{}>", col.collection_id, col.label),
                None => "Details".to_string(),
            };

            let url = match &result.collection {
                Some(col) => col.links.ui.clone(),
                None => "N/A".to_string(),
            };

            let body = format!(
                "Total: {} | Active: {} | Finished: {}\nRemaining time: {}\nTook: {}\nURL: {}",
                result.total,
                result.active,
                result.finished,
                result.remaining_time.as_ref().unwrap_or(&"N/A".to_string()),
                result.took.as_ref().unwrap_or(&"N/A".to_string()),
                url
            );

            let info_block = Block::default()
                .title(title)
                .padding(Padding::new(1, 1, 1, 1))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded);
            let info_block = Paragraph::new(body).block(info_block);
            f.render_widget(info_block, chunks[2]);
        }
    }

    f.render_widget(
        Paragraph::new(app.error_message.to_string()).style(Style::new().red()),
        chunks[3],
    );

    let status_bar_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Min(1), Constraint::Min(25)])
        .split(chunks[4]);
    f.render_widget(
        Block::default().title(format!("aleph-tui version {}", app.version)),
        status_bar_chunks[0],
    );
    let fetching_icon = match app.is_fetching {
        true => "🔄",
        false => "",
    };
    let last_fetch = Local::now() - app.last_fetch;
    let last_fetch = last_fetch.human(Truncate::Second);
    let last_fetch_text = format!(
        "{} fetching every {}s - last fetch {} ago",
        fetching_icon, app.config.fetch_interval, last_fetch,
    );
    f.render_widget(
        Block::default()
            .title(last_fetch_text)
            .title_alignment(Alignment::Left),
        status_bar_chunks[1],
    );
    f.render_widget(
        Block::default()
            .title("Shortcuts: `q`, `^C`, `Esc` - quit, `p` - select profile")
            .title_alignment(Alignment::Right),
        status_bar_chunks[2],
    );

    if app.show_profile_selector() {
        let popup_block = Block::default()
            .title("Select profile")
            .borders(Borders::ALL);

        let area = centered_rect(40, 25, f.area());
        f.render_widget(popup_block.clone(), area);

        let mut rows = Vec::new();
        for (idx, profile) in app.config.profiles.clone().into_iter().enumerate() {
            rows.push(Row::new([profile.name.to_string()]));
            if app.current_profile == profile.index {
                app.profile_tablestate.select(Some(idx))
            }
        }
        let profile_table = Table::new(rows, [Constraint::Min(15)])
            .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>");
        f.render_stateful_widget(
            profile_table,
            popup_block.inner(area),
            &mut app.profile_tablestate,
        );
    }
}
