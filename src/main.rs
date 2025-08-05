use std::borrow::Cow;
use std::collections::VecDeque;
use std::error::Error;
use std::fs::{OpenOptions, read_to_string};
use std::io::Write;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::*;
use rmcp::{ServerHandler, serve_server, tool, tool_handler, tool_router};
use rmcp::service::RequestContext;

// Constants to avoid string duplication
const LIGHTBULB_ON_STATUS: &str = "The lightbulb is on";
const LIGHTBULB_OFF_STATUS: &str = "The lightbulb is off";
const LIGHTBULB_ALREADY_ON: &str = "The lightbulb is already on";
const LIGHTBULB_ALREADY_OFF: &str = "The lightbulb is already off";
const LIGHTBULB_TURNED_ON: &str = "Lightbulb turned on successfully";
const LIGHTBULB_TURNED_OFF: &str = "Lightbulb turned off successfully";
const LOG_FILE_NAME: &str = "lightbulb.log";
const LOG_ACTION_ON: &str = "ON";
const LOG_ACTION_OFF: &str = "OFF";

// Trait for logging abstraction
trait Logger {
    fn log_event(&mut self, action: &str) -> Result<(), Box<dyn Error>>;
    fn read_log(&self) -> Result<String, Box<dyn Error>>;
}

// File-based logger for production
struct FileLogger {
    file_path: String,
}

impl FileLogger {
    fn new(file_path: String) -> Self {
        Self { file_path }
    }
}

impl Logger for FileLogger {
    fn log_event(&mut self, action: &str) -> Result<(), Box<dyn Error>> {
        let timestamp = Utc::now();
        let log_entry = format!("[{}] Lightbulb turned {}\n", timestamp.to_rfc3339(), action);
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)?;
        
        file.write_all(log_entry.as_bytes())?;
        Ok(())
    }
    
    fn read_log(&self) -> Result<String, Box<dyn Error>> {
        read_to_string(&self.file_path).map_err(|e| e.into())
    }
}

// In-memory logger for testing
#[derive(Debug, Clone)]
struct InMemoryLogger {
    entries: VecDeque<String>,
}

impl InMemoryLogger {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}

impl Logger for InMemoryLogger {
    fn log_event(&mut self, action: &str) -> Result<(), Box<dyn Error>> {
        let timestamp = Utc::now();
        let log_entry = format!("[{}] Lightbulb turned {}", timestamp.to_rfc3339(), action);
        self.entries.push_back(log_entry);
        Ok(())
    }
    
    fn read_log(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.entries.iter().map(|entry| format!("{}\n", entry)).collect())
    }
}

struct LightService {
    tool_router: ToolRouter<Self>,
    light_state: Arc<Mutex<bool>>,
    logger: Arc<Mutex<Box<dyn Logger + Send>>>,
}

#[tool_router]
impl LightService {
    #[tool(description = "Get the current status of the lightbulb")]
    async fn get_lightbulb_status(&self) -> String {
        let state = self.light_state.lock().unwrap();
        if *state {
            LIGHTBULB_ON_STATUS.to_owned()
        } else {
            LIGHTBULB_OFF_STATUS.to_owned()
        }
    }

    #[tool(description = "Turn on the lightbulb")]
    async fn turn_on_lightbulb(&self) -> Result<String, String> {
        self.change_lightbulb_state(true, LIGHTBULB_ALREADY_ON, LIGHTBULB_TURNED_ON, LOG_ACTION_ON)
    }

    #[tool(description = "Turn off the lightbulb")]
    async fn turn_off_lightbulb(&self) -> Result<String, String> {
        self.change_lightbulb_state(false, LIGHTBULB_ALREADY_OFF, LIGHTBULB_TURNED_OFF, LOG_ACTION_OFF)
    }

    fn change_lightbulb_state(
        &self,
        target_state: bool,
        already_message: &str,
        success_message: &str,
        log_action: &str,
    ) -> Result<String, String> {
        let mut state = self.light_state.lock().unwrap();
        if *state == target_state {
            Ok(already_message.to_owned())
        } else {
            *state = target_state;
            self.log_light_event(log_action).map_err(|e| format!("Failed to log event: {}", e))?;
            Ok(success_message.to_owned())
        }
    }

    fn log_light_event(&self, action: &str) -> Result<(), Box<dyn Error>> {
        let mut logger = self.logger.lock().unwrap();
        logger.log_event(action)
    }

    fn read_log_content(&self) -> Result<String, Box<dyn Error>> {
        let logger = self.logger.lock().unwrap();
        logger.read_log()
    }

    fn generate_usage_summary(&self) -> String {
        match self.read_log_content() {
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

    fn new_with_logger(logger: Box<dyn Logger + Send>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            light_state: Arc::new(Mutex::new(false)),
            logger: Arc::new(Mutex::new(logger)),
        }
    }

    fn new() -> Self {
        // For production, use file logger
        let logger = FileLogger::new(LOG_FILE_NAME.to_string());
        Self::new_with_logger(Box::new(logger) as Box<dyn Logger + Send>)
    }

    #[cfg(test)]
    fn new_with_in_memory_logger() -> Self {
        let logger = InMemoryLogger::new();
        Self::new_with_logger(Box::new(logger))
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
                let content = match self.read_log_content() {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let logger = FileLogger::new(LOG_FILE_NAME.to_string());
    let server = LightService::new_with_logger(Box::new(logger));

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    serve_server(server, transport).await?.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initial_lightbulb_state() {
        let service = LightService::new_with_in_memory_logger();
        let status = service.get_lightbulb_status().await;
        assert_eq!(status, "The lightbulb is off");
    }

    #[tokio::test]
    async fn test_turn_on_lightbulb() {
        let service = LightService::new_with_in_memory_logger();
        let result = service.turn_on_lightbulb().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Lightbulb turned on successfully");

        let status = service.get_lightbulb_status().await;
        assert_eq!(status, "The lightbulb is on");
    }

    #[tokio::test]
    async fn test_turn_off_lightbulb() {
        let service = LightService::new_with_in_memory_logger();
        // First turn it on
        let _ = service.turn_on_lightbulb().await;

        let result = service.turn_off_lightbulb().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Lightbulb turned off successfully");

        let status = service.get_lightbulb_status().await;
        assert_eq!(status, "The lightbulb is off");
    }

    #[tokio::test]
    async fn test_turn_on_already_on() {
        let service = LightService::new_with_in_memory_logger();
        let _ = service.turn_on_lightbulb().await;

        let result = service.turn_on_lightbulb().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "The lightbulb is already on");
    }

    #[tokio::test]
    async fn test_turn_off_already_off() {
        let service = LightService::new_with_in_memory_logger();

        let result = service.turn_off_lightbulb().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "The lightbulb is already off");
    }

    #[tokio::test]
    async fn test_logging_behavior() {
        let service = LightService::new_with_in_memory_logger();
        
        // Turn on the lightbulb
        let _ = service.turn_on_lightbulb().await;
        
        // Turn off the lightbulb
        let _ = service.turn_off_lightbulb().await;
        
        // Check that the log contains both actions
        let log_content = service.read_log_content().unwrap();
        assert!(log_content.contains("turned ON"));
        assert!(log_content.contains("turned OFF"));
    }
}
