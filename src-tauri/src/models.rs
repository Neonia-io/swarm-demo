use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct AgentEvent {
    pub agent_id: String,
    pub status: String,
    pub details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_x: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_y: Option<f64>,
}

#[derive(Clone)]
pub struct PredefComponent {
    pub name: &'static str,
    pub width: f64,
    pub height: f64,
    pub svg_file: &'static str,
}
