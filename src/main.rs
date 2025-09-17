use crossterm::{
	event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dotenvy::dotenv;
use ratatui::{
	backend::{Backend, CrosstermBackend},
	layout::{Constraint, Direction, Layout},
	style::{Color, Modifier, Style},
	text::{Line, Span},
	widgets::{Block, Borders, Paragraph, Wrap},
	Frame, Terminal,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{convert::From, env, error::Error, format, io, mem::drop, string::{String, ToString}, sync::Arc, time::Duration};
use tokio::sync::Mutex;

// ==================================================

#[derive(Serialize, Debug)]
struct RequestPart {
	text: String,
}

#[derive(Serialize, Debug)]
struct RequestContent {
	parts: Vec<RequestPart>,
	role: String,
}

#[derive(Serialize, Debug)]
struct GeminiRequest {
	contents: Vec<RequestContent>,
}

#[derive(Deserialize, Debug)]
struct ResponsePart {
	text: String,
}

#[derive(Deserialize, Debug)]
struct ResponseContent {
	parts: Vec<ResponsePart>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
	content: ResponseContent,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
	candidates: Vec<Candidate>,
}

// ==================================================

enum AppMode {
	Normal,
	Processing,
}

enum Role {
	User,
	Model,
}

struct Message {
	role: Role,
	content: String,
}

struct AppState {
	mode: AppMode,
	input: String,
	messages: Vec<Message>,
	api_key: String,
	http_client: Client,
	scroll: u16,
}

impl AppState {
	fn new(api_key: String) -> Self {
		Self {
			mode: AppMode::Normal,
			input: String::new(),
			messages: vec![],
			api_key,
			http_client: Client::new(),
			scroll: 0,
		}
	}

	async fn send_message(app_state: Arc<Mutex<Self>>) {
		let mut state = app_state.lock().await;
		if state.input.is_empty() { return; }

		let user_message = state.input.drain(..).collect::<String>();
		state.messages.push(Message {
			role: Role::User,
			content: user_message.clone(),
		});
		state.mode = AppMode::Processing;

		let contents: Vec<RequestContent> = state
			.messages
			.iter()
			.map(|msg| {
				let (role, text) = match msg.role {
					Role::User => ("user", &msg.content),
					Role::Model => ("model", &msg.content),
				};
				RequestContent {
					parts: vec![RequestPart { text: text.to_string() }],
					role: role.to_string(),
				}
			})
			.collect();

		let api_key = state.api_key.clone();
		let client = state.http_client.clone();

		drop(state);

		tokio::spawn(async move {
			let result = call_gemini_api(&api_key, &client, contents).await;
			let mut state = app_state.lock().await;

			match result {
				Ok(response_text) => {
					state.messages.push(Message {
						role: Role::Model,
						content: response_text,
					});
				}
				Err(e) => {
					state.messages.push(Message {
						role: Role::Model,
						content: format!("Error: {}", e),
					});
				}
			}
			state.mode = AppMode::Normal;
		});
	}
}

async fn call_gemini_api(
	api_key: &str,
	client: &Client,
	contents: Vec<RequestContent>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
	let endpoint = format!(
		"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
	);

	let request_body = GeminiRequest { contents };
	let res = client.post(&endpoint).json(&request_body).send().await?;

	if res.status().is_success() {
		let response_body: GeminiResponse = res.json().await?;
		if let Some(candidate) = response_body.candidates.first() {
			if let Some(part) = candidate.content.parts.first() {
				return Ok(part.text.clone());
			}
		}
		Err("Invalid response format from API".into())
	} else {
		let error_text = res.text().await?;
		Err(format!("API Error: {}", error_text).into())
	}
}

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

	let res = run_app(&mut terminal, app_state).await;

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

async fn run_app<B: Backend>(
	terminal: &mut Terminal<B>,
	app_state: Arc<Mutex<AppState>>,
) -> io::Result<()> {
	loop {
		let app_guard = app_state.lock().await;
		terminal.draw(|f| ui(f, &app_guard))?;
		drop(app_guard);

		if event::poll(Duration::from_millis(200))? {
			if let Event::Key(key) = event::read()? {

				if key.kind == KeyEventKind::Press {
					let mut state = app_state.lock().await;

					match state.mode {
						AppMode::Normal => match key.code {
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
						AppMode::Processing => {}
					}
				}
			}
		}
	}
}

fn ui(f: &mut Frame, app: &AppState) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
		.split(f.area());

	let mut message_lines: Vec<Line> = Vec::new();
	for m in &app.messages {
		let (prefix, style) = match m.role {
			Role::User => ("You: ", Style::default().fg(Color::Yellow)),
			Role::Model => ("Gemini: ", Style::default().fg(Color::Cyan)),
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
