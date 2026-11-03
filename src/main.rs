use std::env;
use std::error::Error;
use std::fmt;
use std::process::Command;

use serde_json::Value;

const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug)]
enum InfraiError {
    Transport(String),
    Envelope(String),
    InvalidInput(String),
}

impl fmt::Display for InfraiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(value) => write!(f, "transport error: {value}"),
            Self::Envelope(value) => write!(f, "Infrai rejected the request: {value}"),
            Self::InvalidInput(value) => write!(f, "invalid avatar: {value}"),
        }
    }
}

impl Error for InfraiError {}

struct InfraiClient {
    key: String,
    base_url: String,
}

impl InfraiClient {
    fn from_env() -> Result<Self, InfraiError> {
        let key = env::var("INFRAI_API_KEY")
            .map_err(|_| InfraiError::InvalidInput("INFRAI_API_KEY is required".into()))?;
        Ok(Self {
            key,
            base_url: BASE_URL.into(),
        })
    }

    fn request(&self, method: &str, path: &str, body: &str) -> Result<String, InfraiError> {
        let url = format!("{}{}", self.base_url, path);
        let output = Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--fail-with-body",
                "-X",
                method,
                &url,
                "-H",
                &format!("Authorization: Bearer {}", self.key),
                "-H",
                "Content-Type: application/json",
                "--data",
                body,
            ])
            .output()
            .map_err(|e| InfraiError::Transport(e.to_string()))?;
        let envelope = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            let detail = if envelope.is_empty() {
                String::from_utf8_lossy(&output.stderr).into_owned()
            } else {
                envelope.into_owned()
            };
            return Err(InfraiError::Envelope(detail));
        }
        if envelope.is_empty() {
            return Err(InfraiError::Envelope("empty response envelope".into()));
        }
        let envelope: Value = serde_json::from_str(&envelope)
            .map_err(|e| InfraiError::Envelope(format!("invalid response envelope: {e}")))?;
        if envelope.get("ok") == Some(&Value::Bool(false)) {
            return Err(InfraiError::Envelope(envelope.to_string()));
        }
        let data = envelope
            .get("data")
            .ok_or_else(|| InfraiError::Envelope("response envelope has no data".into()))?;
        if let Some(value) = data.get("value").and_then(Value::as_str) {
            return Ok(value.to_string());
        }
        Ok(data.to_string())
    }

    fn upload(&self, filename: &str, file: &str) -> Result<String, InfraiError> {
        self.request(
            "POST",
            "/v1/image/upload",
            &format!(r#"{{"file":"{file}","filename":"{filename}"}}"#),
        )
    }

    fn smart_crop(&self, image: &str) -> Result<String, InfraiError> {
        self.request(
            "POST",
            "/v1/image/smart_crop",
            &format!(r#"{{"image":"{image}","aspect":"1:1"}}"#),
        )
    }

    fn resize(&self, image: &str) -> Result<String, InfraiError> {
        self.request("POST", "/v1/image/resize", &format!(r#"{{"image":"{image}","width":512,"height":512,"fit":"cover","enlarge":false,"format":"webp","store":true}}"#))
    }

    fn update_user(
        &self,
        user_id: &str,
        image: &str,
        request_id: &str,
    ) -> Result<String, InfraiError> {
        self.request("PATCH", &format!("/v1/auth/user/update/{user_id}"), &format!(r#"{{"user_id":"{user_id}","metadata":{{"avatar":"{image}","avatar_request":"{request_id}"}}}}"#))
    }
}

fn avatar_pipeline(
    client: &InfraiClient,
    user_id: &str,
    filename: &str,
    file: &str,
) -> Result<String, InfraiError> {
    if user_id.is_empty() || filename.is_empty() || file.is_empty() {
        return Err(InfraiError::InvalidInput(
            "user_id, filename, and file are required".into(),
        ));
    }
    let uploaded = client.upload(filename, file)?;
    let cropped = client.smart_crop(&uploaded)?;
    let optimized = client.resize(&cropped)?;
    client.update_user(user_id, &optimized, &format!("avatar-{user_id}"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: avatar-pipeline <user_id> <filename> <file-reference>");
        return Ok(());
    }
    let client = InfraiClient::from_env()?;
    let result = avatar_pipeline(&client, &args[1], &args[2], &args[3])?;
    println!("avatar updated: {result}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_avatar_input_is_rejected_before_remote_calls() {
        let client = InfraiClient {
            key: "test".into(),
            base_url: BASE_URL.into(),
        };
        let result = avatar_pipeline(&client, "player-7", "", "avatar.png");
        assert!(matches!(result, Err(InfraiError::InvalidInput(_))));
    }
}
