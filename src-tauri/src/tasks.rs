use colored::*;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai::Client as OpenAiClient;
use rmcp::RoleClient;

use rmcp::model::InitializeRequestParams;
use rmcp::service::{Peer, RunningService};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use uuid::Uuid;

use crate::fetch_resource;
use crate::hooks::DemoHook;
use crate::models::{AgentEvent, PredefComponent};
use crate::utils::{find_valid_position, queue_pop};

pub fn spawn_producer_task(
    app_producer: AppHandle,
    worker_mcp_client: Arc<RunningService<RoleClient, InitializeRequestParams>>,
    q_name_clone: String,
) {
    tokio::spawn(async move {
        println!("{}", "[PRODUCER] Initializing with Static Assets...".cyan());

        let predef_components = [
            PredefComponent {
                name: "IRON MINE",
                width: 140.0,
                height: 140.0,
                svg_file: "iron_mine.svg",
            },
            PredefComponent {
                name: "COPPER MINE",
                width: 140.0,
                height: 140.0,
                svg_file: "copper_mine.svg",
            },
            PredefComponent {
                name: "SMELTER",
                width: 180.0,
                height: 180.0,
                svg_file: "smelter.svg",
            },
            PredefComponent {
                name: "ASSEMBLER",
                width: 200.0,
                height: 200.0,
                svg_file: "assembler.svg",
            },
            PredefComponent {
                name: "WAREHOUSE",
                width: 240.0,
                height: 160.0,
                svg_file: "warehouse.svg",
            },
            PredefComponent {
                name: "POWER PLANT",
                width: 220.0,
                height: 220.0,
                svg_file: "power_plant.svg",
            },
            PredefComponent {
                name: "TRANSPORT HUB",
                width: 120.0,
                height: 120.0,
                svg_file: "transport_hub.svg",
            },
            PredefComponent {
                name: "FLUID PUMP",
                width: 100.0,
                height: 160.0,
                svg_file: "fluid_pump.svg",
            },
            PredefComponent {
                name: "RESEARCH LAB",
                width: 260.0,
                height: 260.0,
                svg_file: "research_lab.svg",
            },
            PredefComponent {
                name: "SOLAR ARRAY",
                width: 160.0,
                height: 240.0,
                svg_file: "solar_array.svg",
            },
        ];

        let mut batch_count = 0;
        loop {
            let _ = app_producer.emit(
                "agent-event",
                AgentEvent {
                    agent_id: "Producer".into(),
                    status: "working".into(),
                    details: "Generating Mechanism Segment...".into(),
                    target_x: None,
                    target_y: None,
                },
            );

            let mut summary = String::new();
            let mut sector_components = vec![predef_components[9].clone()];
            if batch_count % 3 == 0 {
                sector_components.extend([
                    predef_components[0].clone(),
                    predef_components[2].clone(),
                    predef_components[3].clone(),
                    predef_components[4].clone(),
                ]);
            } else if batch_count % 3 == 1 {
                sector_components.extend([
                    predef_components[1].clone(),
                    predef_components[2].clone(),
                    predef_components[3].clone(),
                    predef_components[6].clone(),
                ]);
            } else {
                sector_components.extend([
                    predef_components[7].clone(),
                    predef_components[5].clone(),
                    predef_components[8].clone(),
                    predef_components[4].clone(),
                ]);
            }
            batch_count += 1;

            for comp in sector_components {
                let payload = serde_json::json!({ "name": comp.name, "width": comp.width, "height": comp.height, "svg_file": comp.svg_file });
                let mut params = rmcp::model::CallToolRequestParams::new("neonia_sys_queue_push");
                params.arguments = Some(
                    serde_json::json!({ "topic": q_name_clone, "payload": payload.to_string() })
                        .as_object()
                        .unwrap()
                        .clone(),
                );
                if let Ok(_) = worker_mcp_client.call_tool(params).await {
                    summary.push_str(&format!(
                        "- {} ({}x{})\n",
                        comp.name, comp.width, comp.height
                    ));
                }
            }

            println!(
                "{}",
                format!("[PRODUCER] ✅ Batch Complete:\n{}", summary).cyan()
            );
            let _ = app_producer.emit(
                "agent-event",
                AgentEvent {
                    agent_id: "Producer".into(),
                    status: "done".into(),
                    details: summary,
                    target_x: None,
                    target_y: None,
                },
            );

            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });
}

pub async fn redraw_background_routes(
    neonia_key: String,
    neonia_mcp_url: String,
    router_app_redraw: AppHandle,
    layout_clone_redraw: Arc<Mutex<Vec<serde_json::Value>>>,
    routes_to_redraw: Vec<serde_json::Value>,
    state_len: usize,
) {
    let client = reqwest::Client::new();
    for i in 0..(state_len - 2) {
        let start_node = &routes_to_redraw[i];
        let end_node = &routes_to_redraw[i + 1];

        let mut obstacles = routes_to_redraw.clone();
        obstacles.remove(i + 1);
        obstacles.remove(i);

        let s_x = start_node.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0)
            + start_node
                .get("width")
                .and_then(|v| v.as_f64())
                .unwrap_or(100.0)
                / 2.0;
        let s_y = start_node.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0)
            + start_node
                .get("height")
                .and_then(|v| v.as_f64())
                .unwrap_or(100.0)
                / 2.0;
        let e_x = end_node.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0)
            + end_node
                .get("width")
                .and_then(|v| v.as_f64())
                .unwrap_or(100.0)
                / 2.0;
        let e_y = end_node.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0)
            + end_node
                .get("height")
                .and_then(|v| v.as_f64())
                .unwrap_or(100.0)
                / 2.0;

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "neo_util_pathfinder",
                "arguments": {
                    "start": {"x": s_x, "y": s_y},
                    "end": {"x": e_x, "y": e_y},
                    "obstacles": obstacles,
                    "padding": 40
                }
            }
        });

        if let Ok(res) = client
            .post(&neonia_mcp_url)
            .bearer_auth(&neonia_key)
            .json(&payload)
            .send()
            .await
        {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(text) = json
                    .pointer("/result/content/0/text")
                    .and_then(|v| v.as_str())
                {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(polyline) = parsed.get("svg_polyline").and_then(|u| u.as_str())
                        {
                            let old_polyline = start_node
                                .get("outgoing_route")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            if polyline != old_polyline {
                                let path_d = format!("M {}", polyline.replace(" ", " L "));
                                let route_name = format!("Route Redraw {}", i);
                                let route_id = format!("route_{}", uuid::Uuid::new_v4().simple());
                                let svg_content = format!(
                                    r##"<svg width="100%" height="100%" style="overflow: visible;" xmlns="http://www.w3.org/2000/svg">
                                        <path id="{}" d="{}" fill="none" stroke="#42f5ce" stroke-width="8" stroke-linejoin="round" filter="drop-shadow(0 0 8px rgba(66, 245, 206, 0.8))"/>
                                        <circle r="6" fill="#f542d4 " filter="drop-shadow(0 0 10px #f542d4 )">
                                            <animateMotion dur="3s" repeatCount="indefinite">
                                                <mpath href="#{}"/>
                                            </animateMotion>
                                            <animate attributeName="r" values="6;10;6" dur="1.5s" repeatCount="indefinite" />
                                        </circle>
                                    </svg>"##,
                                    route_id, path_d, route_id
                                );
                                let _ = router_app_redraw.emit("delete-route", route_name.clone());
                                let _ = router_app_redraw.emit("svg-generated", serde_json::json!({ 
                                    "agent_id": "Router Background", 
                                    "svg": svg_content,
                                    "instances": [serde_json::json!({"x": 0.0, "y": 0.0, "size": 100.0})],
                                    "item_name": route_name
                                }));

                                if let Ok(mut state) = layout_clone_redraw.lock() {
                                    if i < state.len() {
                                        state[i]["outgoing_route"] = serde_json::json!(polyline);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn spawn_consumer_task(
    app_host: AppHandle,
    queue_name: String,
    neonia_key: String,
    neonia_mcp_url: String,
    layout_state: Arc<Mutex<Vec<serde_json::Value>>>,
    mcp_client: Arc<RunningService<RoleClient, InitializeRequestParams>>,
    worker_peer: Peer<RoleClient>,
    openrouter_client: OpenAiClient,
    all_tools: Vec<rmcp::model::Tool>,
) {
    tokio::spawn(async move {
        let mut items_processed = 0;
        let mut first_item_popped = false;

        loop {
            sleep(Duration::from_millis(500)).await;
            println!(
                "{}",
                "[RUST HOST] Polling queue... (0 LLM tokens spent)".dimmed()
            );

            if let Some(item) = queue_pop(&queue_name, &neonia_key, &neonia_mcp_url).await {
                items_processed += 1;
                println!(
                    "{}",
                    format!("[RUST HOST] 📦 Popped item from queue: {}", item)
                        .green()
                        .bold()
                );

                let _ = app_host.emit(
                    "queue-popped",
                    json!({ "item": &item, "agent_id": items_processed }),
                );

                let (item_name, item_width, item_height, svg_file) =
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&item) {
                        let name = json
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or(&item)
                            .to_string();
                        let width = json.get("width").and_then(|v| v.as_f64()).unwrap_or(100.0);
                        let height = json.get("height").and_then(|v| v.as_f64()).unwrap_or(100.0);
                        let svg_file = json
                            .get("svg_file")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        (name, width, height, svg_file)
                    } else {
                        (item.clone(), 100.0, 100.0, "".to_string())
                    };

                let layout_clone = layout_state.clone();
                let worker_tools = all_tools.clone();
                let worker_peer = worker_peer.clone();
                let worker_or_client = openrouter_client.clone();
                let agent_id = items_processed;

                let worker_app = app_host.clone();
                let worker_mcp_client = mcp_client.clone();
                let neonia_key_inner = neonia_key.clone();
                let neonia_mcp_url_inner = neonia_mcp_url.clone();

                tokio::spawn(async move {
                    let agent_name = format!("Worker {}", agent_id);
                    let current_layout_json = {
                        let state = layout_clone.lock().unwrap();
                        serde_json::to_string(&*state).unwrap_or_else(|_| "[]".to_string())
                    };

                    let worker_prompt = format!(
                        "You are Spatial Architect {agent_id}. Task: Place the factory block '{item_name}' (Size: {width}x{height}).
                        CURRENT FACTORY LAYOUT: {current_layout}
                        WORKFLOW RULES:
                        1. REASONING: Read the CURRENT FACTORY LAYOUT. Identify the bounding boxes of existing components. Calculate a NEW (x, y) coordinate (between 0-3000 width, 0-3000 height) that provides at least 180px of padding from all existing components. Do NOT guess randomly. Calculate it logically!
                        2. HYPOTHESIS: Output a `[Hypothesis]` block stating your calculated (x, y) coordinates and why they fit.
                        3. SPATIAL VALIDATION: Call `neo_util_svg_layout_validator`. 
                           Pass the CURRENT FACTORY LAYOUT array PLUS YOUR NEW COMPONENT (with `\"allow_overlap\": false`, id: '{item_name}_{agent_id}', your calculated x, y, width: {width}, height: {height}).
                        4. REFLECTION & HEAL: If validation fails (`is_valid: false`), output a `[Reflection]` block, calculate a DIFFERENT empty area, and call the validator AGAIN! 
                           CRITICAL: You MUST loop this step until the validator returns `is_valid: true`.
                        5. FINAL OUTPUT: Do NOT call `neo_util_svg_generator`. Once validated, immediately return EXACTLY this JSON block on a new line at the end, containing your final valid coordinates:
                        {{\"x\": <final_x>, \"y\": <final_y>}}",
                        agent_id = agent_id, item_name = item_name, width = item_width, height = item_height, current_layout = current_layout_json
                    );

                    let agent = worker_or_client
                        .agent("google/gemini-3-flash-preview")
                        .preamble(&worker_prompt)
                        .default_max_turns(15)
                        .rmcp_tools(worker_tools.clone(), worker_peer.clone())
                        .build();

                    let prefix = format!("[AGENT-{}: {}]", agent_id, item_name);
                    println!(
                        "{}",
                        format!("{} 🚀 Spawned! Spatial reasoning...", prefix).yellow()
                    );

                    let _ = worker_app.emit(
                        "agent-event",
                        AgentEvent {
                            agent_id: agent_name.clone(),
                            status: "started".into(),
                            details: format!("Placing {}", item_name),
                            target_x: None,
                            target_y: None,
                        },
                    );

                    match agent
                        .prompt(&format!(
                            "Place the {} on the factory layout and generate its SVG.",
                            item_name
                        ))
                        .with_hook(DemoHook {
                            prefix: prefix.clone(),
                            agent_id: agent_name.clone(),
                            app: worker_app.clone(),
                        })
                        .await
                    {
                        Ok(res) => {
                            let _ = worker_app.emit(
                                "agent-event",
                                AgentEvent {
                                    agent_id: agent_name.clone(),
                                    status: "done".into(),
                                    details: "Finished!".into(),
                                    target_x: None,
                                    target_y: None,
                                },
                            );

                            if let Some(start) = res.find('{') {
                                if let Some(end) = res.rfind('}') {
                                    let json_str = &res[start..=end];
                                    if let Ok(parsed) =
                                        serde_json::from_str::<serde_json::Value>(json_str)
                                    {
                                        let mut new_component = parsed.clone();
                                        let mut final_x =
                                            parsed.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        let mut final_y =
                                            parsed.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);

                                        {
                                            let mut state = layout_clone.lock().unwrap();
                                            let (fx, fy) = find_valid_position(
                                                final_x,
                                                final_y,
                                                item_width,
                                                item_height,
                                                &state,
                                            );
                                            final_x = fx;
                                            final_y = fy;

                                            new_component["x"] = serde_json::json!(final_x);
                                            new_component["y"] = serde_json::json!(final_y);
                                            new_component["id"] = serde_json::json!(format!(
                                                "{}_{}",
                                                item_name, agent_id
                                            ));
                                            new_component["component_id"] = serde_json::json!(
                                                format!("{}_{}", item_name, agent_id)
                                            );
                                            new_component["width"] = serde_json::json!(item_width);
                                            new_component["height"] =
                                                serde_json::json!(item_height);
                                            new_component["allow_overlap"] =
                                                serde_json::json!(false);
                                            state.push(new_component);
                                        }

                                        let svg_content_opt = if !svg_file.is_empty() {
                                            if let Ok(content) = std::fs::read_to_string(format!(
                                                "assets/components/{}",
                                                svg_file
                                            )) {
                                                Some(content)
                                            } else {
                                                None
                                            }
                                        } else if let Some(uri) =
                                            parsed.get("svg_uri").and_then(|u| u.as_str())
                                        {
                                            let clean_uri = uri.trim_matches(|c| {
                                                c == '"'
                                                    || c == '\''
                                                    || c == '.'
                                                    || c == ','
                                                    || c == '`'
                                                    || c == '*'
                                                    || c == '<'
                                                    || c == '>'
                                                    || c == '{'
                                                    || c == '}'
                                                    || c == '['
                                                    || c == ']'
                                            });
                                            fetch_resource!(worker_mcp_client, clean_uri)
                                        } else {
                                            None
                                        };

                                        if let Some(svg_content) = svg_content_opt {
                                            let instances = vec![
                                                serde_json::json!({ "x": final_x, "y": final_y, "size": item_width.max(item_height) }),
                                            ];
                                            let _ = worker_app.emit("svg-generated", json!({ "agent_id": agent_name, "svg": svg_content, "instances": instances, "item_name": item_name }));

                                            let (state_len, routes_to_redraw) = {
                                                let state = layout_clone.lock().unwrap();
                                                (state.len(), state.clone())
                                            };

                                            let previous_block = if state_len >= 2 {
                                                Some(routes_to_redraw[state_len - 2].clone())
                                            } else {
                                                None
                                            };

                                            if state_len >= 3 {
                                                tokio::spawn(redraw_background_routes(
                                                    neonia_key_inner.clone(),
                                                    neonia_mcp_url_inner.clone(),
                                                    worker_app.clone(),
                                                    layout_clone.clone(),
                                                    routes_to_redraw,
                                                    state_len,
                                                ));
                                            }

                                            if let Some(prev) = previous_block {
                                                let router_app = worker_app.clone();
                                                let router_or_client = worker_or_client.clone();
                                                let router_tools = worker_tools.clone();
                                                let router_peer = worker_peer.clone();
                                                let r_mcp_client = worker_mcp_client.clone();
                                                let r_agent_name = format!("Router {}", agent_id);

                                                let current_layout_json = {
                                                    let state = layout_clone.lock().unwrap();
                                                    serde_json::to_string(
                                                        &state
                                                            .iter()
                                                            .take(state.len().saturating_sub(2))
                                                            .collect::<Vec<_>>(),
                                                    )
                                                    .unwrap_or_else(|_| "[]".to_string())
                                                };

                                                let prev_x = prev
                                                    .get("x")
                                                    .and_then(|v| v.as_f64())
                                                    .unwrap_or(0.0);
                                                let prev_y = prev
                                                    .get("y")
                                                    .and_then(|v| v.as_f64())
                                                    .unwrap_or(0.0);
                                                let prev_w = prev
                                                    .get("width")
                                                    .and_then(|v| v.as_f64())
                                                    .unwrap_or(100.0);
                                                let prev_h = prev
                                                    .get("height")
                                                    .and_then(|v| v.as_f64())
                                                    .unwrap_or(100.0);

                                                let start_x = prev_x + prev_w / 2.0;
                                                let start_y = prev_y + prev_h / 2.0;
                                                let end_x = final_x + item_width / 2.0;
                                                let end_y = final_y + item_height / 2.0;

                                                let router_prompt = format!(
                                                        "You are the Logistician Router. Connect the previous block to '{item_name}'.
                                                        CURRENT FACTORY LAYOUT OBSTACLES: {obstacles}
                                                        WORKFLOW RULES:
                                                        1. Call `neo_util_pathfinder` with: start: {{\"x\": {start_x}, \"y\": {start_y}}}, end: {{\"x\": {end_x}, \"y\": {end_y}}}, obstacles: (CURRENT FACTORY LAYOUT), padding: 40
                                                        2. Return EXACTLY: {{\"svg_polyline\": \"<polyline_string>\"}}",
                                                        item_name = item_name, obstacles = current_layout_json, start_x = start_x, start_y = start_y, end_x = end_x, end_y = end_y
                                                    );

                                                tokio::spawn(async move {
                                                    let agent = router_or_client
                                                        .agent("google/gemini-3-flash-preview")
                                                        .preamble(&router_prompt)
                                                        .default_max_turns(5)
                                                        .rmcp_tools(router_tools, router_peer)
                                                        .build();
                                                    let prefix = format!(
                                                        "[ROUTER-{}: {}]",
                                                        agent_id, item_name
                                                    );
                                                    println!(
                                                        "{}",
                                                        format!("{} 🔌 Routing...", prefix)
                                                            .magenta()
                                                    );

                                                    if let Ok(res) = agent
                                                        .prompt("Connect the buildings.")
                                                        .with_hook(DemoHook {
                                                            prefix: prefix.clone(),
                                                            agent_id: r_agent_name.clone(),
                                                            app: router_app.clone(),
                                                        })
                                                        .await
                                                    {
                                                        if let Some(start) = res.find('{') {
                                                            if let Some(end) = res.rfind('}') {
                                                                let json_str = &res[start..=end];
                                                                if let Ok(parsed) =
                                                                    serde_json::from_str::<
                                                                        serde_json::Value,
                                                                    >(
                                                                        json_str
                                                                    )
                                                                {
                                                                    if let Some(polyline) = parsed
                                                                        .get("svg_polyline")
                                                                        .and_then(|u| u.as_str())
                                                                    {
                                                                        let path_d = format!(
                                                                            "M {}",
                                                                            polyline.replace(
                                                                                " ", " L "
                                                                            )
                                                                        );
                                                                        let route_id = format!(
                                                                            "route_{}",
                                                                            Uuid::new_v4().simple()
                                                                        );
                                                                        let svg_content = format!(
                                                                            r##"<svg width="1200" height="800" viewBox="0 0 1200 800" xmlns="http://www.w3.org/2000/svg"><path id="{}" d="{}" fill="none" stroke="#42f5ce" stroke-width="8" stroke-linejoin="round" filter="drop-shadow(0 0 8px rgba(66, 245, 206, 0.8))"/><circle r="6" fill="#f542d4 " filter="drop-shadow(0 0 10px #f542d4 )"><animateMotion dur="3s" repeatCount="indefinite"><mpath href="#{}"/> </animateMotion><animate attributeName="r" values="6;10;6" dur="1.5s" repeatCount="indefinite" /></circle></svg>"##,
                                                                            route_id,
                                                                            path_d,
                                                                            route_id
                                                                        );
                                                                        let _ = router_app.emit("svg-generated", json!({ "agent_id": r_agent_name, "svg": svg_content, "instances": [json!({"x": 0.0, "y": 0.0, "size": 1200.0})], "item_name": format!("Route to {}", item_name) }));
                                                                    } else if let Some(uri) = parsed
                                                                        .get("svg_uri")
                                                                        .and_then(|u| u.as_str())
                                                                    {
                                                                        let clean_uri = uri
                                                                            .trim_matches(|c| {
                                                                                c == '"'
                                                                                    || c == '\''
                                                                                    || c == '.'
                                                                                    || c == ','
                                                                                    || c == '`'
                                                                                    || c == '*'
                                                                                    || c == '<'
                                                                                    || c == '>'
                                                                                    || c == '{'
                                                                                    || c == '}'
                                                                                    || c == '['
                                                                                    || c == ']'
                                                                            });
                                                                        if let Some(svg_content) = fetch_resource!(
                                                                            r_mcp_client,
                                                                            clean_uri
                                                                        ) {
                                                                            let _ = router_app.emit("svg-generated", json!({ "agent_id": r_agent_name, "svg": svg_content, "instances": [json!({"x": 0.0, "y": 0.0, "size": 1200.0})], "item_name": format!("Route to {}", item_name) }));
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = worker_app.emit(
                                "agent-event",
                                AgentEvent {
                                    agent_id: agent_name.clone(),
                                    status: "error".into(),
                                    details: e.to_string(),
                                    target_x: None,
                                    target_y: None,
                                },
                            );
                        }
                    }
                });

                if !first_item_popped {
                    first_item_popped = true;
                    sleep(Duration::from_secs(15)).await;
                }
            }
        }
    });
}
