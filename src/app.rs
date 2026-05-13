use leptos::ev::{MouseEvent, WheelEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use gloo_timers::future::sleep;
use core::time::Duration;
use wasm_bindgen::prelude::*;
use serde::Deserialize;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}

#[derive(Clone)]
struct Agent {
    id: String,
    role: String,
    x: f64,
    y: f64,
    target_x: f64,
    target_y: f64,
}

#[derive(Deserialize)]
struct TauriEvent<T> {
    payload: T,
}

#[derive(Deserialize, Clone)]
struct AgentEventPayload {
    agent_id: String,
    status: String,
    details: String,
    target_x: Option<f64>,
    target_y: Option<f64>,
}

#[derive(Deserialize, Clone)]
struct SvgInstance {
    x: f64,
    y: f64,
    size: f64,
}

#[derive(Deserialize, Clone)]
struct SvgPayload {
    agent_id: String,
    svg: String,
    instances: Option<Vec<SvgInstance>>,
    target_x: Option<f64>,
    target_y: Option<f64>,
    size: Option<f64>,
    item_name: Option<String>,
}

#[component]
pub fn App() -> impl IntoView {
    let (pan_x, set_pan_x) = signal(0.0f64);
    let (pan_y, set_pan_y) = signal(0.0f64);
    let (scale, set_scale) = signal(1.0f64);
    let (is_panning, set_is_panning) = signal(false);
    let (last_mouse, set_last_mouse) = signal((0.0, 0.0));

    let (logs, set_logs) = signal(Vec::<String>::new());
    let (producer_logs, set_producer_logs) = signal(Vec::<String>::new());
    let (reasoning_logs, set_reasoning_logs) = signal(Vec::<String>::new());
    let (is_swarm_running, set_swarm_running) = signal(false);
    // (x, y, size, svg_str, item_name, part_index)
    let (svgs, set_svgs) = signal(Vec::<(f64, f64, f64, String, String, usize)>::new());
    let (part_types, set_part_types) = signal(Vec::<String>::new());

    let (agents, set_agents) = signal(Vec::<Agent>::new());
    let (svg_count, set_svg_count) = signal(0);
    
    let (api_cost, set_api_cost) = signal(0.0f64);
    let (wasm_cost, set_wasm_cost) = signal(0.0f64);

    // Simulated animation loop
    Effect::new(move |_| {
        spawn_local(async move {
            loop {
                sleep(Duration::from_millis(16)).await; // ~60fps
                
                set_agents.update(|agents| {
                    if !is_swarm_running.get_untracked() {
                        return; // Do not move before swarm starts
                    }

                    for agent in agents.iter_mut() {
                        let dx = agent.target_x - agent.x;
                        let dy = agent.target_y - agent.y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        
                        if dist < 2.0 {
                            // Jitter slightly around the target instead of roaming far
                            agent.target_x = agent.target_x + (js_sys::Math::random() - 0.5) * 20.0;
                            agent.target_y = agent.target_y + (js_sys::Math::random() - 0.5) * 20.0;
                        } else {
                            // Move towards target
                            let speed = 4.0;
                            agent.x += (dx / dist) * speed;
                            agent.y += (dy / dist) * speed;
                        }
                    }
                });
            }
        });
    });

    // Tauri Listeners setup
    Effect::new(move |_| {
        let closure_agent_event = Closure::wrap(Box::new(move |val: JsValue| {
            match serde_wasm_bindgen::from_value::<TauriEvent<AgentEventPayload>>(val) {
                Ok(event) => {
                    if event.payload.agent_id == "Producer" {
                        set_producer_logs.update(|logs| {
                            logs.push(format!("> {}: {}", event.payload.status, event.payload.details));
                            if logs.len() > 15 { logs.remove(0); }
                        });
                    } else {
                        set_logs.update(|logs| {
                            logs.push(format!("[{}] {}: {}", event.payload.agent_id, event.payload.status, event.payload.details));
                            if logs.len() > 10 { logs.remove(0); }
                        });

                        // Dynamically spawn workers
                        if event.payload.status == "started" {
                            let mut current_agents = agents.get_untracked();
                            if !current_agents.iter().any(|a| a.id == event.payload.agent_id) {
                                let idx = svg_count.get_untracked();
                                set_svg_count.set(idx + 1);
                                
                                let target_x = event.payload.target_x.unwrap_or_else(|| js_sys::Math::random() * 3000.0);
                                let target_y = event.payload.target_y.unwrap_or_else(|| js_sys::Math::random() * 3000.0);
                                
                                current_agents.push(Agent {
                                    id: event.payload.agent_id.clone(),
                                    role: event.payload.agent_id.clone(),
                                    x: target_x - 100.0, // spawn slightly offset
                                    y: target_y - 100.0,
                                    target_x,
                                    target_y,
                                });
                                set_agents.set(current_agents);
                                // Initial context prompt cost
                                set_api_cost.update(|c| *c += 0.0015);
                            }
                        } else if event.payload.status == "working" {
                            if let (Some(tx), Some(ty)) = (event.payload.target_x, event.payload.target_y) {
                                let mut current_agents = agents.get_untracked();
                                if let Some(agent) = current_agents.iter_mut().find(|a| a.id == event.payload.agent_id) {
                                    agent.target_x = tx;
                                    agent.target_y = ty;
                                }
                                set_agents.set(current_agents);
                            }
                        } else if event.payload.status == "done" || event.payload.status == "error" {
                            let mut current_agents = agents.get_untracked();
                            current_agents.retain(|a| a.id != event.payload.agent_id);
                            set_agents.set(current_agents);
                        }
                    }
                }
                Err(e) => {
                    leptos::logging::error!("Deserialize agent-event error: {:?}", e);
                }
            }
        }) as Box<dyn FnMut(JsValue)>);

        spawn_local(async move {
            listen("agent-event", &closure_agent_event).await;
            closure_agent_event.forget(); // leak it so it stays alive
        });

        let closure_svg = Closure::wrap(Box::new(move |val: JsValue| {
            match serde_wasm_bindgen::from_value::<TauriEvent<SvgPayload>>(val) {
                Ok(event) => {
                    set_svgs.update(|svgs| {
                        let mut clean_svg = event.payload.svg;
                        clean_svg = clean_svg.replace("fill=\"#FFFFFF\"", "fill=\"none\"");
                        clean_svg = clean_svg.replace("fill=\"#ffffff\"", "fill=\"none\"");
                        clean_svg = clean_svg.replace("background-color: white", "background-color: transparent");

                        let item_name = event.payload.item_name.clone().unwrap_or_else(|| "Component".to_string());
                        
                        if !item_name.starts_with("Route") {
                            if let Some(idx) = clean_svg.find('>') {
                                clean_svg.insert_str(idx + 1, "<rect width=\"100%\" height=\"100%\" fill=\"rgba(10, 15, 25, 0.95)\" rx=\"8\" stroke=\"none\" />");
                            }
                        }
                        
                        let mut p_types = part_types.get_untracked();
                        let part_index = if let Some(idx) = p_types.iter().position(|x| x == &item_name) {
                            idx + 1
                        } else {
                            p_types.push(item_name.clone());
                            set_part_types.set(p_types.clone());
                            p_types.len()
                        };

                        if let Some(instances) = event.payload.instances {
                            for inst in instances {
                                svgs.push((inst.x, inst.y, inst.size, clean_svg.clone(), item_name.clone(), part_index));
                            }
                        } else {
                            let size = event.payload.size.unwrap_or(256.0);
                            let target_x = event.payload.target_x.unwrap_or_else(|| {
                                let current_agents = agents.get_untracked();
                                if let Some(agent) = current_agents.iter().find(|a| a.id == event.payload.agent_id) {
                                    agent.target_x
                                } else {
                                    js_sys::Math::random() * 800.0
                                }
                            });
                            let target_y = event.payload.target_y.unwrap_or_else(|| {
                                let current_agents = agents.get_untracked();
                                if let Some(agent) = current_agents.iter().find(|a| a.id == event.payload.agent_id) {
                                    agent.target_y
                                } else {
                                    js_sys::Math::random() * 600.0
                                }
                            });
                            svgs.push((target_x, target_y, size, clean_svg, item_name, part_index));
                        }
                    });
                }
                Err(e) => {
                    leptos::logging::error!("Deserialize svg-generated error: {:?}", e);
                }
            }
        }) as Box<dyn FnMut(JsValue)>);

        let closure_reasoning = Closure::wrap(Box::new(move |val: JsValue| {
            if let Some(event_str) = val.as_string() {
                set_reasoning_logs.update(|logs| {
                    logs.push(event_str);
                    if logs.len() > 15 { logs.remove(0); }
                });
                set_api_cost.update(|c| *c += 0.0005);
            } else if let Ok(event) = serde_wasm_bindgen::from_value::<TauriEvent<String>>(val) {
                set_reasoning_logs.update(|logs| {
                    logs.push(event.payload.clone());
                    if logs.len() > 15 { logs.remove(0); }
                });
                set_api_cost.update(|c| *c += 0.0005);
            }
        }) as Box<dyn FnMut(JsValue)>);

        let closure_tool_call = Closure::wrap(Box::new(move |_val: JsValue| {
            // Approx cost of executing a single Wasm component locally or in a serverless worker
            set_wasm_cost.update(|c| *c += 0.000001);
        }) as Box<dyn FnMut(JsValue)>);

        let closure_clear_routes = Closure::wrap(Box::new(move |_val: JsValue| {
            set_svgs.update(|svgs| {
                svgs.retain(|(_, _, _, _, name, _)| !name.starts_with("Route"));
            });
        }) as Box<dyn FnMut(JsValue)>);

        let closure_delete_route = Closure::wrap(Box::new(move |val: JsValue| {
            if let Some(route_name) = val.as_string() {
                set_svgs.update(|svgs| {
                    svgs.retain(|(_, _, _, _, name, _)| name != &route_name);
                });
            }
        }) as Box<dyn FnMut(JsValue)>);

        spawn_local(async move {
            listen("svg-generated", &closure_svg).await;
            closure_svg.forget();
            listen("reasoning-stream", &closure_reasoning).await;
            closure_reasoning.forget();
            listen("tool_call", &closure_tool_call).await;
            closure_tool_call.forget();
            listen("clear-routes", &closure_clear_routes).await;
            closure_clear_routes.forget();
            listen("delete-route", &closure_delete_route).await;
            closure_delete_route.forget();
        });
    });

    let on_mouse_down = move |ev: MouseEvent| {
        if ev.button() == 1 || ev.button() == 0 {
            set_is_panning.set(true);
            set_last_mouse.set((ev.client_x() as f64, ev.client_y() as f64));
        }
    };

    let on_mouse_up = move |_ev: MouseEvent| {
        set_is_panning.set(false);
    };

    let on_mouse_leave = move |_ev: MouseEvent| {
        set_is_panning.set(false);
    };

    let on_mouse_move = move |ev: MouseEvent| {
        if is_panning.get() {
            let cx = ev.client_x() as f64;
            let cy = ev.client_y() as f64;
            let (lx, ly) = last_mouse.get();
            let dx = cx - lx;
            let dy = cy - ly;

            set_pan_x.update(|x| *x += dx);
            set_pan_y.update(|y| *y += dy);
            set_last_mouse.set((cx, cy));
        }
    };

    let on_wheel = move |ev: WheelEvent| {
        ev.prevent_default();
        let dy = ev.delta_y();
        let zoom_factor = if dy > 0.0 { 0.9 } else { 1.1 };
        
        let mx = ev.client_x() as f64;
        let my = ev.client_y() as f64;
        
        let mut curr_scale = scale.get();
        let mut curr_pan_x = pan_x.get();
        let mut curr_pan_y = pan_y.get();
        
        let world_x = (mx - curr_pan_x) / curr_scale;
        let world_y = (my - curr_pan_y) / curr_scale;
        
        curr_scale *= zoom_factor;
        curr_scale = curr_scale.clamp(0.05, 10.0);
        
        curr_pan_x = mx - world_x * curr_scale;
        curr_pan_y = my - world_y * curr_scale;
        
        set_scale.set(curr_scale);
        set_pan_x.set(curr_pan_x);
        set_pan_y.set(curr_pan_y);
    };

    let start_swarm_cmd = move |_| {
        set_swarm_running.set(true);
        spawn_local(async move {
            invoke("start_swarm", JsValue::NULL).await;
        });
    };

    view! {
        <div 
            class="canvas-container"
            on:mousedown=on_mouse_down
            on:mousemove=on_mouse_move
            on:mouseup=on_mouse_up
            on:mouseleave=on_mouse_leave
            on:wheel=on_wheel
            on:contextmenu=move |ev| ev.prevent_default()
        >
            // Grid background that moves and scales with the pan
            <div 
                style=move || format!(
                    "position: absolute; top: 0; left: 0; width: 100vw; height: 100vh; background-image: radial-gradient(var(--grid-color) 1px, transparent 1px); background-size: {}px {}px; background-position: {}px {}px;",
                    50.0 * scale.get(), 50.0 * scale.get(), pan_x.get(), pan_y.get()
                )
            ></div>

            <div 
                class="infinite-canvas"
                style=move || format!(
                    "transform: translate({}px, {}px) scale({}); transform-origin: 0 0;",
                    pan_x.get(), pan_y.get(), scale.get()
                )
            >
                // Render SVGs
                {move || {
                    let svg_list = svgs.get();
                    svg_list.clone().into_iter().map(|(x, y, _size, svg_str, name, _index)| {
                        let z_index = if name.starts_with("Route") { 0 } else { 10 };
                        view! {
                            <div>
                                <div 
                                    style=format!("position: absolute; left: 0px; top: 0px; transform: translate({tx}px, {ty}px); transform-origin: top left; display: flex; justify-content: center; align-items: center; z-index: {z};", tx=x, ty=y, z=z_index)
                                    inner_html=svg_str
                                ></div>
                            </div>
                        }
                    }).collect_view()
                }}

                // Render Agents
                {move || {
                    agents.get().into_iter().map(|agent| {
                        view! {
                            <div 
                                class="agent-cursor"
                                style=format!("transform: translate({}px, {}px);", agent.x, agent.y)
                            >
                                <div class="agent-dot"></div>
                                <div class="agent-label">{agent.role}</div>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>
            
            <div class="ui-overlay">
                <div class="producer-panel">
                    <h2 class="panel-title" style="color: var(--accent-color); font-size: 14px;">"Planner Agent (Producer)"</h2>
                    <div style="font-family: monospace; font-size: 11px; opacity: 0.9; display: flex; flex-direction: column; gap: 4px; overflow-y: auto; max-height: 200px;">
                        {move || producer_logs.get().into_iter().map(|log| {
                            view! { <div style="margin-bottom: 2px;">{log}</div> }
                        }).collect_view()}
                        {move || if producer_logs.get().is_empty() {
                            view! { <div style="opacity: 0.5;">"Waiting for Swarm to start..."</div> }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>
                </div>

                // Reasoning Stream Panel
                <div class="reasoning-panel" style="position: absolute; left: 20px; top: 20px; width: 340px; background: rgba(10, 15, 25, 0.85); border: 1px solid var(--text-color); border-radius: 8px; padding: 15px; color: var(--text-color); font-family: monospace; font-size: 12px; z-index: 1000; box-shadow: 0 0 15px rgba(66,245,206,0.2); backdrop-filter: blur(5px);">
                    <h3 style="margin: 0 0 10px 0; font-size: 14px; text-transform: uppercase; border-bottom: 1px solid var(--text-color); padding-bottom: 5px;">"Architect Reasoning Stream"</h3>
                    <div style="display: flex; flex-direction: column; gap: 8px; max-height: 400px; overflow-y: auto;">
                        {move || reasoning_logs.get().into_iter().map(|log| {
                            let bg = if log.contains("⚠️") { "rgba(255, 50, 50, 0.15)" } else { "rgba(66, 245, 206, 0.1)" };
                            let border = if log.contains("⚠️") { "#ff3333" } else { "var(--text-color)" };
                            view! {
                                <div style=format!("padding: 8px; background: {}; border-left: 2px solid {}; border-radius: 2px;", bg, border)>
                                    {log}
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>

                <div class="dashboard-panel">
                    <h2 class="panel-title">"Neonia Swarm"</h2>
                    
                    {move || if !is_swarm_running.get() {
                        view! {
                            <button 
                                on:click=start_swarm_cmd 
                                style="width: 100%; padding: 10px; background: var(--text-color); color: #000; border: none; border-radius: 4px; font-weight: bold; cursor: pointer;"
                            >
                                "START SWARM"
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <div style="color: var(--accent-color); font-weight: bold; text-align: center; padding: 10px;">
                                "SWARM ACTIVE"
                            </div>
                        }.into_any()
                    }}

                    <div style="font-size: 13px; opacity: 0.9; margin-top: 15px; display: flex; flex-direction: column; gap: 8px;">
                        <div><strong style="color: var(--accent-color);">"Agents Active:"</strong> " " {move || agents.get().len()}</div>
                        <div><strong style="color: var(--accent-color);">"Zoom Level:"</strong> " " {move || format!("{:.0}%", scale.get() * 100.0)}</div>
                        
                        <div style="margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--grid-color);">
                            <div style="font-size: 11px; opacity: 0.7; margin-bottom: 4px;">"FINANCIAL DASHBOARD"</div>
                            <div style="display: flex; justify-content: space-between;">
                                <span>"API Cost:"</span>
                                <span style="color: #ff4040;">{move || format!("${:.4}", api_cost.get())}</span>
                            </div>
                            <div style="display: flex; justify-content: space-between; margin-top: 4px;">
                                <span>"Wasm Cost:"</span>
                                <span style="color: #42f5ce;">{move || format!("${:.6}", wasm_cost.get())}</span>
                            </div>
                        </div>

                        <div style="margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--grid-color); font-family: monospace; font-size: 10px; word-wrap: break-word; overflow-wrap: break-word; overflow-y: auto; max-height: 250px;">
                            <div style="color: var(--accent-color); margin-bottom: 5px;">"LIVE LOGS"</div>
                            {move || logs.get().into_iter().map(|log| {
                                view! { <div style="margin-bottom: 2px;">{log}</div> }
                            }).collect_view()}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
