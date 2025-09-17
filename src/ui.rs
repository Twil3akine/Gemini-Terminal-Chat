use crate::app;
use crate::app::{AppMode, AppState};
use ratatui::{
	layout::{Constraint, Direction, Layout},
	style::{Color, Modifier, Style},
	text::{Line, Span},
	widgets::{Block, Borders, Paragraph, Wrap},
	Frame,
};

pub fn ui(f: &mut Frame, app: &AppState) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
		.split(f.area());

	let mut message_lines: Vec<Line> = Vec::new();
	for m in &app.messages {
		let (prefix, style) = match m.role {
			app::Role::User => ("You: ", Style::default().fg(Color::Yellow)),
			app::Role::Model => ("Gemini: ", Style::default().fg(Color::Cyan)),
		};

		let line = Line::from(vec![
			Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
			Span::raw(&m.content),
		]);

		message_lines.push(line);
		message_lines.push(Line::from(""));
	}

	let chat_paragraph = Paragraph::new(message_lines)
		.block(Block::default().borders(Borders::ALL).title("Chat (↑↓ scroll)"))
		.style(Style::default())
		.wrap(Wrap { trim: true })
		.scroll((app.scroll, 0));

	f.render_widget(chat_paragraph, chunks[0]);

	let input_title = match app.mode {
		AppMode::Normal => "Input (Enter to send, Esc to quit)",
		AppMode::Processing => "Processing...",
	};
	let input = Paragraph::new(app.input.as_str())
		.style(match app.mode {
			AppMode::Normal => Style::default(),
			AppMode::Processing => Style::default().fg(Color::DarkGray),
		})
		.block(Block::default().borders(Borders::ALL).title(input_title));
	f.render_widget(input, chunks[1]);
}