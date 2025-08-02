use std::error::Error;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router, serve_server};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use chrono::Utc;

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
            drop(state); // Release the lock before logging
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
            drop(state); // Release the lock before logging
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
        file.flush()?;
        Ok(())
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
                .enable_logging()
                .build(),
            ..Default::default()
        }
    }
}
