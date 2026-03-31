use std::io::{self, BufRead, BufReader, BufWriter, Write};

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    adaptation::{AdaptiveResetRequest, InteractionRecordRequest},
    app::deps::SoulDependencies,
    domain::{InteractionEvent, PersonalityProfilePatch, SoulConfigPatch, SoulError},
    mcp::tools,
    storage::sqlite::ResetScope,
};

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const MODERN_PROTOCOL_VERSION: &str = "2025-03-26";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpServer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MessageMode {
    Framed,
    LineDelimited,
}

impl McpServer {
    pub fn serve_stdio(&self, deps: &SoulDependencies) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = BufWriter::new(stdout.lock());

        while let Some((body, message_mode)) = read_message(&mut reader)? {
            let response = match serde_json::from_str::<Value>(&body) {
                Ok(request) => self.handle_request(deps, request),
                Err(error) => Some(jsonrpc_error(
                    Value::Null,
                    -32700,
                    format!("failed to parse json-rpc request: {error}"),
                    None,
                )),
            };

            if let Some(response) = response {
                write_message(&mut writer, &response, message_mode)?;
            }
        }

        Ok(())
    }

    fn handle_request(&self, deps: &SoulDependencies, request: Value) -> Option<Value> {
        let id = request.get("id").cloned();
        let method = match request.get("method").and_then(Value::as_str) {
            Some(method) => method,
            None => {
                return Some(jsonrpc_error(
                    id.unwrap_or(Value::Null),
                    -32600,
                    "json-rpc request must include a string method",
                    None,
                ));
            }
        };

        match method {
            "initialize" => Some(jsonrpc_result(
                id.unwrap_or(Value::Null),
                json!({
                    "protocolVersion": negotiated_protocol_version(&request),
                    "serverInfo": {
                        "name": "agents-soul",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                    "capabilities": {
                        "tools": {},
                    },
                }),
            )),
            "notifications/initialized" => None,
            "ping" => Some(jsonrpc_result(id.unwrap_or(Value::Null), json!({}))),
            "tools/list" => Some(jsonrpc_result(
                id.unwrap_or(Value::Null),
                json!({ "tools": tool_definitions() }),
            )),
            "tools/call" => {
                let request_id = match id {
                    Some(id) => id,
                    None => {
                        return Some(jsonrpc_error(
                            Value::Null,
                            -32600,
                            "tools/call requests must include an id",
                            None,
                        ));
                    }
                };

                Some(match self.call_tool(deps, request.get("params")) {
                    Ok(payload) => jsonrpc_result(
                        request_id,
                        json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": payload,
                                }
                            ],
                            "isError": false,
                        }),
                    ),
                    Err(error) => error.into_response(request_id),
                })
            }
            _ => id.map(|request_id| {
                jsonrpc_error(
                    request_id,
                    -32601,
                    format!("unsupported MCP method `{method}`"),
                    None,
                )
            }),
        }
    }

    fn call_tool(
        &self,
        deps: &SoulDependencies,
        params: Option<&Value>,
    ) -> Result<String, McpCallError> {
        let params = params.unwrap_or(&Value::Null);
        let tool_name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params("tools/call params.name must be a string"))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        match tool_name {
            "compose_context" => {
                let request = parse_args(arguments, "compose_context arguments")?;
                let payload = tools::compose_context(deps, request)?;
                serialize_payload(&payload)
            }
            "get_prefix" => {
                let request = parse_args(arguments, "get_prefix arguments")?;
                let payload = tools::get_prefix(deps, request)?;
                serialize_payload(&payload)
            }
            "explain_report" => {
                let request = parse_args(arguments, "explain_report arguments")?;
                let payload = tools::explain_report(deps, request)?;
                serialize_payload(&payload)
            }
            "get_traits" => {
                let request = parse_args(arguments, "get_traits arguments")?;
                let payload = tools::get_traits(deps, request)?;
                serialize_payload(&payload)
            }
            "get_heuristics" => {
                let request = parse_args(arguments, "get_heuristics arguments")?;
                let payload = tools::get_heuristics(deps, request)?;
                serialize_payload(&payload)
            }
            "configure_workspace" => {
                let args: ConfigureWorkspaceArgs =
                    parse_args(arguments, "configure_workspace arguments")?;
                let payload = tools::configure_workspace(deps, args.workspace_root, args.patch)?;
                serialize_payload(&payload)
            }
            "update_traits" => {
                let args: UpdateTraitsArgs = parse_args(arguments, "update_traits arguments")?;
                let payload = tools::update_traits(deps, args.workspace_root, args.patch)?;
                serialize_payload(&payload)
            }
            "record_interaction" => {
                let args: RecordInteractionArgs =
                    parse_args(arguments, "record_interaction arguments")?;
                let payload = tools::record_interaction(
                    deps,
                    args.workspace_root,
                    InteractionRecordRequest {
                        event_id: args.event_id,
                        event: args.event,
                        context_json: args.context_json,
                        persist: args.persist,
                    },
                )?;
                serialize_payload(&payload)
            }
            "reset_adaptation_state" => {
                let args: ResetAdaptationArgs =
                    parse_args(arguments, "reset_adaptation_state arguments")?;
                let payload = tools::reset_adaptation_state(
                    deps,
                    args.workspace_root,
                    AdaptiveResetRequest {
                        reset_id: args.reset_id,
                        agent_id: args.agent_id,
                        scope: parse_reset_scope(&args.scope)?,
                        target_key: args.target_key,
                        notes: args.notes,
                        recorded_at: args.recorded_at.unwrap_or_else(Utc::now),
                    },
                )?;
                serialize_payload(&payload)
            }
            _ => Err(invalid_params(format!(
                "unknown MCP tool `{tool_name}`; use tools/list to discover supported tools"
            ))),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigureWorkspaceArgs {
    workspace_root: String,
    patch: SoulConfigPatch,
}

#[derive(Debug, Deserialize)]
struct UpdateTraitsArgs {
    workspace_root: String,
    patch: PersonalityProfilePatch,
}

#[derive(Debug, Deserialize)]
struct RecordInteractionArgs {
    workspace_root: String,
    event_id: String,
    event: InteractionEvent,
    #[serde(default)]
    context_json: String,
    #[serde(default = "default_true")]
    persist: bool,
}

#[derive(Debug, Deserialize)]
struct ResetAdaptationArgs {
    workspace_root: String,
    reset_id: String,
    agent_id: String,
    scope: String,
    #[serde(default)]
    target_key: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    recorded_at: Option<DateTime<Utc>>,
}

enum McpCallError {
    JsonRpc {
        code: i64,
        message: String,
        data: Option<Value>,
    },
    Tool(crate::SoulMcpToolError),
}

impl McpCallError {
    fn into_response(self, id: Value) -> Value {
        match self {
            Self::JsonRpc {
                code,
                message,
                data,
            } => jsonrpc_error(id, code, message, data),
            Self::Tool(error) => jsonrpc_error(
                id,
                i64::from(error.data.error_code),
                error.message,
                Some(json!({
                    "code": error.code,
                    "category": error.data.category,
                    "http_status": error.data.http_status,
                    "cli_exit_code": error.data.cli_exit_code,
                    "compose_mode_hint": error.data.compose_mode_hint,
                })),
            ),
        }
    }
}

impl From<SoulError> for McpCallError {
    fn from(error: SoulError) -> Self {
        Self::Tool(tools::tool_error(&error))
    }
}

fn serialize_payload<T>(value: &T) -> Result<String, McpCallError>
where
    T: serde::Serialize,
{
    serde_json::to_string(value).map_err(|error| McpCallError::JsonRpc {
        code: -32603,
        message: format!("failed to serialize MCP tool payload: {error}"),
        data: None,
    })
}

fn parse_args<T: DeserializeOwned>(arguments: Value, label: &str) -> Result<T, McpCallError> {
    serde_json::from_value(arguments).map_err(|error| {
        invalid_params(format!(
            "{label} must match the shared agents-soul contract: {error}"
        ))
    })
}

fn invalid_params(message: impl Into<String>) -> McpCallError {
    McpCallError::JsonRpc {
        code: -32602,
        message: message.into(),
        data: None,
    }
}

fn parse_reset_scope(scope: &str) -> Result<ResetScope, McpCallError> {
    match scope {
        "all" => Ok(ResetScope::All),
        "trait" => Ok(ResetScope::Trait),
        "communication" => Ok(ResetScope::Communication),
        "heuristic" => Ok(ResetScope::Heuristic),
        other => Err(invalid_params(format!(
            "reset scope must be one of all, trait, communication, heuristic; got `{other}`"
        ))),
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "compose_context",
            "Build the shared behavioral context using ComposeRequest fields.",
            contract_schema("ComposeRequest"),
        ),
        tool(
            "get_prefix",
            "Return only the system prompt prefix using ComposeRequest fields.",
            contract_schema("ComposeRequest"),
        ),
        tool(
            "explain_report",
            "Build the explain report using ComposeRequest fields.",
            contract_schema("ComposeRequest"),
        ),
        tool(
            "get_traits",
            "Read the trait projection using ComposeRequest fields.",
            contract_schema("ComposeRequest"),
        ),
        tool(
            "get_heuristics",
            "Read the heuristic projection using ComposeRequest fields.",
            contract_schema("ComposeRequest"),
        ),
        tool(
            "configure_workspace",
            "Apply a SoulConfigPatch to a workspace using { workspace_root, patch }.",
            json_schema(
                &[
                    ("workspace_root", json!({"type": "string"})),
                    (
                        "patch",
                        json!({
                            "type": "object",
                            "description": "SoulConfigPatch fields"
                        }),
                    ),
                ],
                &["workspace_root", "patch"],
            ),
        ),
        tool(
            "update_traits",
            "Apply a PersonalityProfilePatch using { workspace_root, patch }.",
            json_schema(
                &[
                    ("workspace_root", json!({"type": "string"})),
                    (
                        "patch",
                        json!({
                            "type": "object",
                            "description": "PersonalityProfilePatch fields"
                        }),
                    ),
                ],
                &["workspace_root", "patch"],
            ),
        ),
        tool(
            "record_interaction",
            "Record adaptive interaction evidence using { workspace_root, event_id, event, context_json?, persist? }.",
            json_schema(
                &[
                    ("workspace_root", json!({"type": "string"})),
                    ("event_id", json!({"type": "string"})),
                    (
                        "event",
                        json!({
                            "type": "object",
                            "description": "InteractionEvent fields"
                        }),
                    ),
                    ("context_json", json!({"type": "string"})),
                    ("persist", json!({"type": "boolean"})),
                ],
                &["workspace_root", "event_id", "event"],
            ),
        ),
        tool(
            "reset_adaptation_state",
            "Reset adaptive state using { workspace_root, reset_id, agent_id, scope, target_key?, notes?, recorded_at? }.",
            json_schema(
                &[
                    ("workspace_root", json!({"type": "string"})),
                    ("reset_id", json!({"type": "string"})),
                    ("agent_id", json!({"type": "string"})),
                    (
                        "scope",
                        json!({
                            "type": "string",
                            "enum": ["all", "trait", "communication", "heuristic"]
                        }),
                    ),
                    ("target_key", json!({"type": "string"})),
                    ("notes", json!({"type": "string"})),
                    (
                        "recorded_at",
                        json!({
                            "type": "string",
                            "description": "RFC3339 timestamp"
                        }),
                    ),
                ],
                &["workspace_root", "reset_id", "agent_id", "scope"],
            ),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

fn contract_schema(type_name: &str) -> Value {
    json!({
        "type": "object",
        "description": format!("Arguments must be a JSON object matching {type_name}."),
        "additionalProperties": true,
    })
}

fn json_schema(properties: &[(&str, Value)], required: &[&str]) -> Value {
    let mut object = serde_json::Map::new();
    for (name, schema) in properties {
        object.insert((*name).to_string(), schema.clone());
    }

    json!({
        "type": "object",
        "properties": Value::Object(object),
        "required": required,
        "additionalProperties": false,
    })
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Value {
    let mut error = json!({
        "code": code,
        "message": message.into(),
    });
    if let Some(data) = data {
        error["data"] = data;
    }

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error,
    })
}

fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<(String, MessageMode)>> {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Ok(None);
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if content_length.is_none() && trimmed.starts_with('{') {
            return Ok(Some((trimmed.to_string(), MessageMode::LineDelimited)));
        }

        if line == "\r\n" || line == "\n" {
            break;
        }

        if let Some(value) = trimmed
            .split_once(':')
            .and_then(|(name, value)| name.eq_ignore_ascii_case("content-length").then_some(value))
        {
            let parsed = value.trim().parse::<usize>().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid Content-Length header `{value}`: {error}"),
                )
            })?;
            content_length = Some(parsed);
        }
    }

    let content_length = content_length.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length header in MCP stdio request",
        )
    })?;
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body)?;
    String::from_utf8(body)
        .map(|body| Some((body, MessageMode::Framed)))
        .map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid UTF-8 body in MCP stdio request: {error}"),
        )
    })
}

fn write_message<W: Write>(
    writer: &mut W,
    message: &Value,
    message_mode: MessageMode,
) -> io::Result<()> {
    let body = serde_json::to_string(message).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize MCP response: {error}"),
        )
    })?;

    match message_mode {
        MessageMode::Framed => {
            write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
            writer.write_all(body.as_bytes())?;
        }
        MessageMode::LineDelimited => {
            writer.write_all(body.as_bytes())?;
            writer.write_all(b"\n")?;
        }
    }

    writer.flush()
}

fn default_true() -> bool {
    true
}

fn negotiated_protocol_version(request: &Value) -> &'static str {
    match request
        .get("params")
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str)
    {
        Some(MODERN_PROTOCOL_VERSION) => MODERN_PROTOCOL_VERSION,
        Some(DEFAULT_PROTOCOL_VERSION) => DEFAULT_PROTOCOL_VERSION,
        _ => DEFAULT_PROTOCOL_VERSION,
    }
}
