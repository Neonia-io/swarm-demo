use tauri::{AppHandle, Emitter};
use colored::*;
use rig::agent::{PromptHook, ToolCallHookAction, HookAction};
use rig::completion::CompletionModel;
use crate::models::AgentEvent;

#[derive(Clone)]
pub struct DemoHook {
    pub prefix: String,
    pub agent_id: String,
    pub app: AppHandle,
}

impl DemoHook {
    pub fn emit(&self, status: &str, details: &str) {
        let msg = format!("[{}] {}: {}", self.agent_id, status, details);
        println!("{}", msg.cyan());
        let _ = self.app.emit("agent-event", AgentEvent {
            agent_id: self.agent_id.clone(),
            status: status.to_string(),
            details: details.to_string(),
            target_x: None,
            target_y: None,
        });
    }

    pub fn emit_reasoning(&self, details: &str) {
        let msg = format!("[{}] {}", self.agent_id, details);
        println!("{}", msg.green());
        let _ = self.app.emit("reasoning-stream", msg);
    }
}

impl<M: CompletionModel> PromptHook<M> for DemoHook {
    async fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
    ) -> ToolCallHookAction {
        match tool_name {
            "neo_util_svg_generator" => {
                self.emit("tool_call", "Generating SVG outline...");
            }
            "neo_util_svg_validator" => {
                self.emit("tool_call", "Running deterministic geometric validation...");
            }
            "neonia_sys_memory_lesson" => {
                self.emit("tool_call", "Writing Architectural Lesson to Swarm Memory...");
            }
            "neonia_sys_memory_search" => {
                self.emit("tool_call", "Searching Global Hive-Mind for geometric patterns...");
            }
            _ => {
                self.emit("tool_call", &format!("Executing {}", tool_name));
            }
        }
        ToolCallHookAction::Continue
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
        result: &str,
    ) -> HookAction {
        match tool_name {
            "neo_util_svg_layout_validator" => {
                if result.contains("\"is_valid\":false") {
                    self.emit_reasoning("[Reflection] ⚠️ Collision detected! Adjusting coordinates...");
                    self.emit("tool_result", "⚠️ Collision detected! Recalculating coordinates...");
                } else if result.contains("\"is_valid\":true") {
                    self.emit_reasoning("[Action] ✨ Valid placement found. Locking coordinates.");
                    self.emit("tool_result", "✨ Space found! Coordinates locked.");
                }
            }
            "neo_util_pathfinder" => {
                self.emit_reasoning("[Action] 🔌 Routing optimal path...");
                self.emit("tool_result", "🔌 Path routed.");
            }
            "neonia_sys_memory_lesson" => {
                self.emit("tool_result", "🌐 Lesson propagated to all agents.");
            }
            _ => {}
        }
        HookAction::Continue
    }
}
