use crate::{app::AppState, ui};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, Terminal};
use std::{io, sync::Arc, time::Duration};
use tokio::sync::Mutex;

pub async fn run<B: Backend>(
	terminal: &mut Terminal<B>,
	app_state: Arc<Mutex<AppState>>,
) -> io::Result<()> {
	loop {
		let app_guard = app_state.lock().await;
		terminal.draw(|f| ui::ui(f, &app_guard))?;
		drop(app_guard);

		if event::poll(Duration::from_millis(200))? {
			if let Event::Key(key) = event::read()? {

				if key.kind == KeyEventKind::Press {
					let mut state = app_state.lock().await;

					match state.mode {
						crate::app::AppMode::Normal => match key.code {
							KeyCode::Char(c) => {
								state.input.push(c);
							}
							KeyCode::Backspace => {
								state.input.pop();
							}
							KeyCode::Enter => {
								drop(state);
								AppState::send_message(app_state.clone()).await;
							}
							KeyCode::Esc => {
								return Ok(());
							}
							KeyCode::Up => {
								if state.scroll > 0 {
									state.scroll -= 1;
								}
							}
							KeyCode::Down => {
								state.scroll += 1;
							}
							_ => {}
						},
						crate::app::AppMode::Processing => {}
					}
				}
			}
		}
	}
}