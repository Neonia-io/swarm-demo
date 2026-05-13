use reqwest::Client;
use serde_json::json;

pub async fn queue_pop(queue_name: &str, neonia_key: &str, mcp_url: &str) -> Option<String> {
    let client = Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "neo_sys_queue_pop",
            "arguments": {
                "topic": queue_name
            }
        }
    });

    let res = client.post(mcp_url).bearer_auth(neonia_key).json(&payload).send().await.ok()?;
    let json: serde_json::Value = res.json().await.ok()?;
    let content_text = json.pointer("/result/content/0/text")?.as_str()?;

    if content_text.contains("empty") || content_text.trim().is_empty() {
        None
    } else {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content_text) {
            if let Some(payload) = parsed.get("payload").and_then(|p| p.as_str()) {
                return Some(payload.to_string());
            }
        }
        Some(content_text.to_string())
    }
}

pub fn find_valid_position(mut final_x: f64, mut final_y: f64, item_width: f64, item_height: f64, state: &[serde_json::Value]) -> (f64, f64) {
    let original_x = final_x;
    let original_y = final_y;
    let mut attempts = 0;
    let mut has_overlap = true;
    
    while has_overlap && attempts < 1000 {
        has_overlap = false;
        for obj in state.iter() {
            let ox = obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let oy = obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let ow = obj.get("width").and_then(|v| v.as_f64()).unwrap_or(100.0);
            let oh = obj.get("height").and_then(|v| v.as_f64()).unwrap_or(100.0);
            
            let padding = 90.0;
            if final_x < ox + ow + padding && final_x + item_width + padding > ox &&
               final_y < oy + oh + padding && final_y + item_height + padding > oy {
                   has_overlap = true;
                   let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
                   let random_val = ((nanos.wrapping_add(attempts as u128 * 1234567)) % 1000) as f64 / 1000.0;
                   let angle = random_val * std::f64::consts::PI * 2.0;
                   let radius = 150.0 + (attempts as f64) * 20.0; 
                   final_x = original_x + angle.cos() * radius;
                   final_y = original_y + angle.sin() * radius;
                   
                   final_x = final_x.clamp(0.0, 3000.0);
                   final_y = final_y.clamp(0.0, 3000.0);
                   attempts += 1;
                   break;
            }
        }
    }
    (final_x, final_y)
}

#[macro_export]
macro_rules! fetch_resource {
    ($mcp_client:expr, $uri:expr) => {{
        let params = rmcp::model::ReadResourceRequestParams::new($uri);
        match $mcp_client.read_resource(params).await {
            Ok(res) => {
                if let Some(content) = res.contents.first() {
                    if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = content {
                        Some(text.clone())
                    } else {
                        println!("[fetch_resource] Error: Resource contents empty or not text");
                        None
                    }
                } else {
                    println!("[fetch_resource] Error: Resource contents empty or not text");
                    None
                }
            }
            Err(e) => {
                println!("[fetch_resource] Error reading resource: {:?}", e);
                None
            }
        }
    }};
}
