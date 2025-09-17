use crate::api::{call_gemini_api, RequestContent, RequestPart};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;

pub enum AppMode {
	Normal,
	Processing,
}

pub enum Role {
	User,
	Model,
}

pub struct Message {
	pub role: Role,
	pub content: String,
}

pub struct AppState {
	pub mode: AppMode,
	pub input: String,
	pub messages: Vec<Message>,
	pub api_key: String,
	pub http_client: Client,
	pub scroll: u16,
}

impl AppState {
	pub fn new(api_key: String) -> Self {
		Self {
			mode: AppMode::Normal,
			input: String::new(),
			messages: vec![],
			api_key,
			http_client: Client::new(),
			scroll: 0,
		}
	}

	pub async fn send_message(app_state: Arc<Mutex<Self>>) {
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