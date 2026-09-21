use opencode_sdk::{Client, CreateSessionRequest, PromptPart, PromptRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .base_url("http://127.0.0.1:4096")
        .build()?;

    let session = client
        .session()
        .create(&CreateSessionRequest::default())
        .await?;

    let response = client
        .session()
        .prompt(
            &session.id,
            &PromptRequest {
                parts: vec![PromptPart::text("Hello from Rust")],
                ..Default::default()
            },
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&response.info)?);
    Ok(())
}
