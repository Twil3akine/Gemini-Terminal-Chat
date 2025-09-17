pub mod api;
pub mod app;
pub mod tui;
pub mod ui;

use crate::app::AppState;
use crossterm::{
	event::{DisableMouseCapture, EnableMouseCapture},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, error::Error, io, sync::Arc};
use tokio::sync::Mutex;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	dotenv().ok();
	let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set in .env file");

	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	let app_state = Arc::new(Mutex::new(AppState::new(api_key)));

	let res = tui::run(&mut terminal, app_state).await;

	disable_raw_mode()?;
	execute!(
		terminal.backend_mut(),
		LeaveAlternateScreen,
		DisableMouseCapture
	)?;
	terminal.show_cursor()?;

	if let Err(err) = res {
		println!("{:?}", err)
	}

	Ok(())
}