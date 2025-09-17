use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize, Debug)]
pub struct RequestPart {
	pub text: String,
}

#[derive(Serialize, Debug)]
pub struct RequestContent {
	pub parts: Vec<RequestPart>,
	pub role: String,
}

#[derive(Serialize, Debug)]
pub struct GeminiRequest {
	pub contents: Vec<RequestContent>,
}

#[derive(Deserialize, Debug)]
pub struct ResponsePart {
	pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
	pub content: ResponseContent,
}

#[derive(Deserialize, Debug)]
pub struct ResponseContent {
	pub parts: Vec<ResponsePart>,
}
#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
	pub candidates: Vec<Candidate>,
}

// Gemini API 呼び出し
pub async fn call_gemini_api(
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