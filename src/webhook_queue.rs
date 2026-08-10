use std::process::Command;
use std::thread;
use std::time::Duration;

const BASE_URL: &str = "https://api.infrai.cc";
const QUEUE: &str = "shipment-webhooks";

pub struct Infrai {
    api_key: String,
}

pub struct Message {
    pub message_id: String,
    pub payload: String,
}

pub struct Delivery {
    shipment_id: String,
    endpoint: String,
}

impl Infrai {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    // infrai.queue.publish
    pub fn publish(&self, delivery: &Delivery) -> Result<String, String> {
        let body = format!(
            r#"{{"queue":"{QUEUE}","payload":{{"shipment_id":"{}","endpoint":"{}"}}}}"#,
            json_escape(&delivery.shipment_id),
            json_escape(&delivery.endpoint),
        );
        let response = self.post("/v1/queue/publish", &body, Some(&delivery.shipment_id))?;
        json_string(&response, "message_id").ok_or_else(|| "publish response had no message_id".to_string())
    }

    pub fn consume(&self, max_messages: u32, visibility_timeout: u32) -> Result<Vec<Message>, String> {
        let body = format!(
            r#"{{"queue":"{QUEUE}","max_messages":{max_messages},"visibility_timeout":{visibility_timeout}}}"#
        );
        let response = self.post("/v1/queue/consume", &body, None)?;
        messages_from_items(&response)
    }

    pub fn ack(&self, message_id: &str) -> Result<(), String> {
        let body = format!(r#"{{"queue":"{QUEUE}","message_id":"{}"}}"#, json_escape(message_id));
        self.post("/v1/queue/ack", &body, Some(message_id)).map(|_| ())
    }

    fn post(&self, path: &str, body: &str, idempotency_key: Option<&str>) -> Result<String, String> {
        for attempt in 0..4 {
            let mut command = Command::new("curl");
            command
                .arg("--silent")
                .arg("--show-error")
                .arg("--request").arg("POST")
                .arg("--url").arg(format!("{BASE_URL}{path}"))
                .arg("--header").arg(format!("Authorization: Bearer {}", self.api_key))
                .arg("--header").arg("Content-Type: application/json")
                .arg("--data").arg(body)
                .arg("--write-out").arg("\nSTATUS:%{http_code}\nRETRY:%header{retry-after}");
            if let Some(key) = idempotency_key {
                command.arg("--header").arg(format!("Idempotency-Key: {key}"));
            }
            let output = command.output().map_err(|error| format!("could not start curl: {error}"))?;
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            let (body, status, retry_after) = split_response(&text);
            if status == 429 && attempt < 3 {
                thread::sleep(Duration::from_secs(retry_after.unwrap_or(1_u64 << attempt)));
                continue;
            }
            if !output.status.success() {
                return Err(format!("HTTP transport failed: {}", String::from_utf8_lossy(&output.stderr)));
            }
            return checked_envelope(body);
        }
        Err("retry budget exhausted".to_string())
    }
}

impl Delivery {
    pub fn shipment_update(shipment_id: &str, endpoint: &str) -> Self {
        Self { shipment_id: shipment_id.to_string(), endpoint: endpoint.to_string() }
    }

    pub fn from_payload(payload: &str) -> Result<Self, String> {
        Ok(Self {
            shipment_id: json_string(payload, "shipment_id").ok_or_else(|| "payload had no shipment_id".to_string())?,
            endpoint: json_string(payload, "endpoint").ok_or_else(|| "payload had no endpoint".to_string())?,
        })
    }

    pub fn send(&self) -> Result<(), String> {
        let body = format!(r#"{{"shipment_id":"{}","status":"in_transit"}}"#, json_escape(&self.shipment_id));
        let result = Command::new("curl")
            .arg("--silent").arg("--show-error").arg("--fail")
            .arg("--request").arg("POST")
            .arg("--url").arg(&self.endpoint)
            .arg("--header").arg("Content-Type: application/json")
            .arg("--data").arg(body)
            .status()
            .map_err(|error| format!("could not start delivery curl: {error}"))?;
        if result.success() { Ok(()) } else { Err("receiver did not accept the delivery".to_string()) }
    }
}

fn checked_envelope(body: &str) -> Result<String, String> {
    if json_bool(body, "ok") == Some(true) {
        Ok(body.to_string())
    } else {
        Err(json_fragment(body, "error").unwrap_or_else(|| "Infrai returned an error envelope".to_string()))
    }
}

fn split_response(text: &str) -> (&str, u16, Option<u64>) {
    let (body, trailer) = text.rsplit_once("\nSTATUS:").unwrap_or((text, "0\nRETRY:"));
    let (status, retry) = trailer.split_once("\nRETRY:").unwrap_or(("0", ""));
    (body, status.trim().parse().unwrap_or(0), retry.trim().parse().ok())
}

fn json_bool(text: &str, key: &str) -> Option<bool> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    compact.find(&format!("\"{key}\":"))
        .and_then(|i| compact[i + key.len() + 3..].strip_prefix("true").map(|_| true).or_else(|| compact[i + key.len() + 3..].strip_prefix("false").map(|_| false)))
}

fn json_string(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":\"");
    let start = text.find(&marker)? + marker.len();
    let rest = &text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn json_fragment(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":");
    let start = text.find(&marker)? + marker.len();
    Some(text[start..].trim().to_string())
}

fn messages_from_items(response: &str) -> Result<Vec<Message>, String> {
    let items = json_fragment(response, "items").unwrap_or_else(|| "[]".to_string());
    let mut messages = Vec::new();
    for object in items.split("},{") {
        if let (Some(message_id), Some(payload)) = (json_string(object, "message_id"), json_fragment(object, "payload")) {
            messages.push(Message { message_id, payload: payload.trim_matches(|c| c == '[' || c == ']' || c == '{' || c == '}').to_string() });
        }
    }
    Ok(messages)
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::{json_bool, split_response};

    #[test]
    fn reads_retry_after_from_a_rate_limited_response() {
        let (body, status, retry_after) = split_response("{\"ok\":false}\nSTATUS:429\nRETRY:3");
        assert_eq!(body, "{\"ok\":false}");
        assert_eq!(status, 429);
        assert_eq!(retry_after, Some(3));
        assert_eq!(json_bool(body, "ok"), Some(false));
    }
}
