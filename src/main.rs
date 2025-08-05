use std::error::Error;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::*;
use rmcp::{ServerHandler, tool, tool_handler, tool_router, serve_server};
use rmcp::service::RequestContext;
use std::fs::{OpenOptions, read_to_string};
use std::io::Write;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use std::borrow::Cow;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    let server = LightService::new();

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    serve_server(server, transport).await?.waiting().await?;
    Ok(())
}

struct LightService {
    tool_router: ToolRouter<Self>,
    light_state: Arc<Mutex<bool>>,
}

#[tool_router]
impl LightService {
    #[tool(description = "Get the current status of the lightbulb")]
    async fn get_lightbulb_status(&self) -> String {
        let state = self.light_state.lock().unwrap();
        if *state {
            "The lightbulb is on".to_owned()
        } else {
            "The lightbulb is off".to_owned()
        }
    }

    #[tool(description = "Turn on the lightbulb")]
    async fn turn_on_lightbulb(&self) -> Result<String, String> {
        let mut state = self.light_state.lock().unwrap();
        if *state {
            Ok("The lightbulb is already on".to_owned())
        } else {
            *state = true;
            self.log_light_event("ON").map_err(|e| format!("Failed to log event: {}", e))?;
            Ok("Lightbulb turned on successfully".to_owned())
        }
    }

    #[tool(description = "Turn off the lightbulb")]
    async fn turn_off_lightbulb(&self) -> Result<String, String> {
        let mut state = self.light_state.lock().unwrap();
        if !*state {
            Ok("The lightbulb is already off".to_owned())
        } else {
            *state = false;
            self.log_light_event("OFF").map_err(|e| format!("Failed to log event: {}", e))?;
            Ok("Lightbulb turned off successfully".to_owned())
        }
    }

    fn log_light_event(&self, action: &str) -> Result<(), Box<dyn Error>> {
        let timestamp = Utc::now();
        let log_entry = format!("[{}] Lightbulb turned {}\n", timestamp.to_rfc3339(), action);
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("lightbulb.log")?;
        
        file.write_all(log_entry.as_bytes())?;
        Ok(())
    }

    fn generate_usage_summary(&self) -> String {
        match read_to_string("lightbulb.log") {
            Ok(log_content) => {
                let lines: Vec<&str> = log_content.lines().filter(|line| !line.trim().is_empty()).collect();
                
                if lines.is_empty() {
                    return "Lightbulb Usage Summary:\n\nNo activity recorded yet.".to_string();
                }
                
                let total_actions = lines.len();
                let on_actions = lines.iter().filter(|line| line.contains("turned ON")).count();
                let off_actions = lines.iter().filter(|line| line.contains("turned OFF")).count();
                
                let current_state = self.light_state.lock().unwrap();
                let current_status = if *current_state { "ON" } else { "OFF" };
                
                // Get first and last action timestamps
                let first_action = lines.first().map(|line| {
                    line.split(']').next().unwrap_or("").trim_start_matches('[').to_string()
                });
                let last_action = lines.last().map(|line| {
                    line.split(']').next().unwrap_or("").trim_start_matches('[').to_string()
                });
                
                format!(
                    "Lightbulb Usage Summary:\n\n\
                    Current Status: {}\n\
                    Total Actions: {}\n\
                    - Turn ON actions: {} ({:.1}%)\n\
                    - Turn OFF actions: {} ({:.1}%)\n\n\
                    Activity Period:\n\
                    - First action: {}\n\
                    - Last action: {}\n\n\
                    Recent Activity (last 5 actions):\n{}",
                    current_status,
                    total_actions,
                    on_actions,
                    if total_actions > 0 { (on_actions as f64 / total_actions as f64) * 100.0 } else { 0.0 },
                    off_actions,
                    if total_actions > 0 { (off_actions as f64 / total_actions as f64) * 100.0 } else { 0.0 },
                    first_action.unwrap_or("N/A".to_string()),
                    last_action.unwrap_or("N/A".to_string()),
                    lines.iter().rev().take(5).rev().map(|line| format!("  {}", line)).collect::<Vec<_>>().join("\n")
                )
            },
            Err(_) => "Lightbulb Usage Summary:\n\nLog file not found. No activity recorded yet.".to_string(),
        }
    }

    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            light_state: Arc::new(Mutex::new(false)),
        }
    }
}

#[tool_handler]
impl ServerHandler for LightService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Service for managing lights".into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_logging()
                .build(),
            ..Default::default()
        }
    }

    async fn list_resources(
        	&self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let resources = vec![
            Resource {
                raw: RawResource {
                    uri: "lightbulb://log".to_string(),
                    name: "Lightbulb Activity Log".to_string(),
                    description: Some("Complete history of lightbulb on/off actions with timestamps".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    size: None,
                },
                annotations: None,
            },
            Resource {
                raw: RawResource {
                    uri: "lightbulb://summary".to_string(),
                    name: "Lightbulb Usage Summary".to_string(),
                    description: Some("Summary statistics of lightbulb usage patterns".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    size: None,
                },
                annotations: None,
            },
        ];
        
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        match request.uri.as_str() {
            "lightbulb://log" => {
                let content = match read_to_string("lightbulb.log") {
                    Ok(log_content) => {
                        if log_content.trim().is_empty() {
                            "No lightbulb activity recorded yet.".to_string()
                        } else {
                            format!("Lightbulb Activity Log:\n\n{}", log_content)
                        }
                    },
                    Err(_) => "Lightbulb log file not found. No activity recorded yet.".to_string(),
                };
                
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(content, &request.uri)],
                })
            },
            "lightbulb://summary" => {
                let summary = self.generate_usage_summary();
                
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(summary, &request.uri)],
                })
            },
            _ => Err(ErrorData {
                code: ErrorCode(-32602),
                message: Cow::Borrowed("Unknown resource URI"),
                data: None,
            }),
        }
    }
}
