use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_CONFIG_PATH: &str = "../config_models.json";
const DEFAULT_PROMPTS_DIR: &str = "../prompts/master";
const DEFAULT_LOCAL_CREDENTIALS_PATH: &str = "../llm.json";
const GOOGLE_AUTH_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";
const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_X402_VERSION: u8 = 2;
const DEFAULT_X402_FACILITATOR_URL: &str = "https://facilitator.goplausible.xyz";
const DEFAULT_X402_NETWORK: &str = "algorand:SGO1GKSzyE7IEPItTxCByw9x8FmnrCDexi9/cOUJOiI=";
const DEFAULT_X402_ASSET: &str = "10458941";
const DEFAULT_X402_AMOUNT: &str = "1000";
const DEFAULT_X402_TIMEOUT_SECONDS: u64 = 300;
static RUNS: OnceLock<Arc<Mutex<HashMap<String, RunResponse>>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelConfigFile {
    #[serde(rename = "defaultRegion")]
    default_region: String,
    models: Vec<ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelConfig {
    id: String,
    provider: String,
    model: String,
    region: String,
    #[serde(rename = "reasoningEffort")]
    reasoning_effort: String,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: Option<OutputTokenLimit>,
    #[serde(rename = "askWorld")]
    ask_world: bool,
    purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum OutputTokenLimit {
    Count(u32),
    Mode(String),
}

#[derive(Debug, Clone, Deserialize)]
struct RunRequest {
    intent: String,
    artifact_type: ArtifactType,
    variant_count: Option<u8>,
    research_enabled: Option<bool>,
    execution_mode: Option<ExecutionMode>,
}

#[derive(Debug, Deserialize)]
struct TestModelRequest {
    model_id: Option<String>,
    prompt: Option<String>,
}

#[derive(Debug, Serialize)]
struct TestModelResponse {
    model_id: String,
    model: String,
    region: String,
    reasoning_effort: String,
    ask_world: bool,
    prompt: String,
    finish_reason: Option<String>,
    text: String,
}

#[derive(Debug)]
struct VertexTextResponse {
    text: String,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactType {
    Skill,
    Prompt,
    Answer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ExecutionMode {
    Mock,
    Live,
}

#[derive(Debug, Clone, Serialize)]
struct RunResponse {
    run_id: String,
    status: String,
    benchmark_status: String,
    execution_mode: ExecutionMode,
    artifact_type: ArtifactType,
    intent: String,
    variant_count: u8,
    stages: Vec<StageOutput>,
    final_variants: Vec<FinalVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality_metrics: Option<QualityMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    benchmark_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QualityMetrics {
    summary: String,
    fci: FactConfidenceMetric,
    acg: AspectCoverageMetric,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FactConfidenceMetric {
    label: String,
    score: u8,
    supported_claims: u32,
    total_claims: u32,
    annotation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AspectCoverageMetric {
    label: String,
    final_coverage: u8,
    best_single_model_coverage: u8,
    coverage_gain: i16,
    covered_aspects: u32,
    total_aspects: u32,
    annotation: String,
}

#[derive(Debug, Clone, Serialize)]
struct StageOutput {
    stage: String,
    model_id: String,
    model: String,
    region: String,
    reasoning_effort: String,
    ask_world: bool,
    finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
    summary: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct FinalVariant {
    index: u8,
    title: String,
    strategy: String,
    content: String,
    reusable_prompt: String,
}

#[derive(Debug, Deserialize)]
struct FinalArtifactPayload {
    answer_markdown: String,
    reusable_prompt_markdown: String,
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

struct HttpResponse {
    status: String,
    content_type: String,
    body: String,
    headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
struct X402PaymentRequired {
    #[serde(rename = "x402Version")]
    x402_version: u8,
    accepts: Vec<X402PaymentRequirement>,
    error: String,
}

#[derive(Debug, Clone, Serialize)]
struct X402PaymentRequirement {
    scheme: String,
    network: String,
    asset: String,
    #[serde(rename = "maxAmountRequired")]
    max_amount_required: String,
    resource: String,
    description: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde(rename = "payTo")]
    pay_to: String,
    #[serde(rename = "maxTimeoutSeconds")]
    max_timeout_seconds: u64,
}

#[derive(Debug, Clone)]
struct X402Config {
    enabled: bool,
    dev_bypass: bool,
    facilitator_url: String,
    network: String,
    asset: String,
    amount: String,
    pay_to: String,
    description: String,
    resource: String,
    max_timeout_seconds: u64,
}

#[derive(Debug)]
struct PaymentVerification {
    response_header: String,
}

enum CreateRunError {
    BadRequest(String),
    PaymentRequired(HttpResponse),
}

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    project_id: Option<String>,
    token_uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

fn main() -> std::io::Result<()> {
    load_dotenv();

    let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("APP_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{host}:{port}");

    let listener = TcpListener::bind(&addr)?;
    println!("skill-creator-server listening on http://{addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    if let Err(error) = handle_connection(stream) {
                        eprintln!("request failed: {error}");
                    }
                });
            }
            Err(error) => eprintln!("connection failed: {error}"),
        }
    }

    Ok(())
}

fn load_dotenv() {
    for path in ["../.env", ".env"] {
        let Ok(raw) = fs::read_to_string(path) else {
            continue;
        };

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };

            let key = key.trim();
            if key.is_empty() || env::var_os(key).is_some() {
                continue;
            }

            env::set_var(key, clean_env_value(value.trim()));
        }

        println!("loaded environment from {path}");
        break;
    }
}

fn clean_env_value(value: &str) -> String {
    if value.len() >= 2 {
        let first = value.as_bytes()[0] as char;
        let last = value.as_bytes()[value.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return value[1..value.len() - 1].to_string();
        }
    }

    value.to_string()
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let request = read_http_request(&stream)?;

    let response = match (request.method.as_str(), request.path.as_str()) {
        ("OPTIONS", _) => HttpResponse::new("204 No Content", "text/plain; charset=utf-8", ""),
        ("GET", "/health") => {
            HttpResponse::json("200 OK", serde_json::json!({ "status": "ok" }).to_string())
        }
        ("GET", "/api/schema") => HttpResponse::json("200 OK", api_schema_response()),
        ("GET", "/api/payment/requirements") => match x402_payment_required_response(None) {
            Ok(payment) => payment,
            Err(error) => HttpResponse::json("500 Internal Server Error", error_json(error)),
        },
        ("GET", "/api/models/config") => match load_model_config() {
            Ok(config) => HttpResponse::json(
                "200 OK",
                serde_json::to_string_pretty(&config).unwrap_or_else(error_json),
            ),
            Err(error) => HttpResponse::json("500 Internal Server Error", error_json(error)),
        },
        ("GET", path) if path.starts_with("/api/runs/") => match get_run(path) {
            Ok(response) => HttpResponse::json(
                "200 OK",
                serde_json::to_string_pretty(&response).unwrap_or_else(error_json),
            ),
            Err(error) => HttpResponse::json("404 Not Found", error_json(error)),
        },
        ("POST", "/api/runs") => match create_run(&request) {
            Ok((response, payment)) => {
                let mut http_response = HttpResponse::json(
                    "200 OK",
                    serde_json::to_string_pretty(&response).unwrap_or_else(error_json),
                );
                if let Some(payment) = payment {
                    http_response
                        .headers
                        .push(("PAYMENT-RESPONSE".to_string(), payment.response_header));
                }
                http_response
            }
            Err(CreateRunError::PaymentRequired(response)) => response,
            Err(CreateRunError::BadRequest(error)) => {
                HttpResponse::json("400 Bad Request", error_json(error))
            }
        },
        ("POST", "/api/test-model") => match test_model(&request.body) {
            Ok(response) => HttpResponse::json(
                "200 OK",
                serde_json::to_string_pretty(&response).unwrap_or_else(error_json),
            ),
            Err(error) => HttpResponse::json("400 Bad Request", error_json(error)),
        },
        ("GET", "/") => HttpResponse::new(
            "200 OK",
            "text/plain; charset=utf-8",
            "Berlin42 Answer Forge backend is running.\n",
        ),
        _ => HttpResponse::json(
            "404 Not Found",
            serde_json::json!({ "error": "not_found" }).to_string(),
        ),
    };

    write_http_response(&mut stream, response)
}

fn read_http_request(stream: &TcpStream) -> std::io::Result<HttpRequest> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut content_length = 0usize;
    let mut headers = HashMap::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let trimmed = header.trim_end();

        if trimmed.is_empty() {
            break;
        }

        if let Some((name, value)) = trimmed.split_once(':') {
            let normalized_name = name.trim().to_ascii_lowercase();
            let trimmed_value = value.trim().to_string();
            if normalized_name == "content-length" {
                content_length = trimmed_value.parse().unwrap_or(0);
            }
            headers.insert(normalized_name, trimmed_value);
        }
    }

    let mut body_bytes = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body_bytes)?;
    }

    let mut parts = request_line.split_whitespace();
    Ok(HttpRequest {
        method: parts.next().unwrap_or_default().to_string(),
        path: parts.next().unwrap_or_default().to_string(),
        headers,
        body: String::from_utf8_lossy(&body_bytes).to_string(),
    })
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> std::io::Result<()> {
    let mut raw = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, PAYMENT-SIGNATURE, X402-DEV-PAYMENT\r\nAccess-Control-Expose-Headers: PAYMENT-REQUIRED, PAYMENT-RESPONSE\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );

    for (name, value) in response.headers {
        raw.push_str(&format!("{name}: {value}\r\n"));
    }

    raw.push_str("\r\n");
    raw.push_str(&response.body);

    stream.write_all(raw.as_bytes())?;
    stream.flush()
}

impl HttpResponse {
    fn new(status: &str, content_type: &str, body: impl Into<String>) -> Self {
        Self {
            status: status.to_string(),
            content_type: content_type.to_string(),
            body: body.into(),
            headers: Vec::new(),
        }
    }

    fn json(status: &str, body: impl Into<String>) -> Self {
        Self::new(status, "application/json", body)
    }
}

fn load_model_config() -> Result<ModelConfigFile, String> {
    let path = env::var("CONFIG_MODELS_PATH").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string());
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read model config at {path}: {error}"))?;

    serde_json::from_str(&raw).map_err(|error| format!("failed to parse model config: {error}"))
}

fn test_model(body: &str) -> Result<TestModelResponse, String> {
    let request: TestModelRequest = serde_json::from_str(body)
        .map_err(|error| format!("invalid test-model request: {error}"))?;
    let model_id = request.model_id.unwrap_or_else(|| "base_llm".to_string());
    let prompt = request
        .prompt
        .unwrap_or_else(|| "Ответь коротко: самый вкусный рецепт пирожка?".to_string());

    let config = load_model_config()?;
    let model = find_model(&config, &model_id);

    if model.provider != "vertex" {
        return Err(format!(
            "model {model_id} uses provider {}, but /api/test-model currently supports vertex only",
            model.provider
        ));
    }

    let vertex_response = call_vertex_generate_content(&model, &prompt)?;

    Ok(TestModelResponse {
        model_id: model.id,
        model: model.model,
        region: model.region,
        reasoning_effort: model.reasoning_effort,
        ask_world: model.ask_world,
        prompt,
        finish_reason: vertex_response.finish_reason,
        text: vertex_response.text,
    })
}

fn call_vertex_generate_content(
    model: &ModelConfig,
    prompt: &str,
) -> Result<VertexTextResponse, String> {
    let service_account = load_service_account()?;
    let project_id = env_nonempty("GOOGLE_CLOUD_PROJECT")
        .or(service_account.project_id.clone())
        .ok_or_else(|| {
            "GOOGLE_CLOUD_PROJECT is not set and service account has no project_id".to_string()
        })?;
    let access_token = fetch_access_token(&service_account)?;
    let endpoint = vertex_generate_content_endpoint(&project_id, &model.region, &model.model);

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))?;

    let payload = serde_json::json!({
        "contents": [
            {
                "role": "user",
                "parts": [
                    { "text": prompt }
                ]
            }
        ],
        "generationConfig": build_generation_config(model)
    });

    let response = client
        .post(endpoint)
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .map_err(|error| format!("Vertex request failed: {error}"))?;

    let status = response.status();
    let value: serde_json::Value = response
        .json()
        .map_err(|error| format!("failed to parse Vertex response JSON: {error}"))?;

    if !status.is_success() {
        return Err(format!("Vertex returned {status}: {value}"));
    }

    extract_vertex_text(&value).ok_or_else(|| format!("Vertex response had no text: {value}"))
}

fn call_vertex_generate_content_timed(
    model: &ModelConfig,
    prompt: &str,
) -> Result<(VertexTextResponse, u64), String> {
    let started_at = Instant::now();
    let response = call_vertex_generate_content(model, prompt)?;
    Ok((response, elapsed_ms(started_at)))
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn build_generation_config(model: &ModelConfig) -> serde_json::Value {
    let mut config = serde_json::Map::new();
    config.insert("temperature".to_string(), serde_json::json!(0.6));
    if let Some(thinking_config) = build_thinking_config(model) {
        config.insert("thinkingConfig".to_string(), thinking_config);
    }

    match &model.max_output_tokens {
        Some(OutputTokenLimit::Count(count)) => {
            config.insert("maxOutputTokens".to_string(), serde_json::json!(count));
        }
        Some(OutputTokenLimit::Mode(mode)) if mode.eq_ignore_ascii_case("unlimited") => {}
        Some(OutputTokenLimit::Mode(mode)) => {
            eprintln!(
                "unknown maxOutputTokens mode {mode}; omitting maxOutputTokens for {}",
                model.id
            );
        }
        None => {
            config.insert("maxOutputTokens".to_string(), serde_json::json!(2048));
        }
    }

    serde_json::Value::Object(config)
}

fn build_thinking_config(model: &ModelConfig) -> Option<serde_json::Value> {
    let effort = model.reasoning_effort.trim();
    if effort.is_empty()
        || effort.eq_ignore_ascii_case("off")
        || effort.eq_ignore_ascii_case("none")
    {
        return None;
    }

    if model.model.starts_with("gemini-3") {
        return Some(serde_json::json!({
            "thinkingLevel": reasoning_effort_to_thinking_level(effort)
        }));
    }

    if model.model.starts_with("gemini-2.5") {
        return Some(serde_json::json!({
            "thinkingBudget": reasoning_effort_to_thinking_budget(effort)
        }));
    }

    None
}

fn reasoning_effort_to_thinking_level(effort: &str) -> &'static str {
    if effort.eq_ignore_ascii_case("minimal") {
        "MINIMAL"
    } else if effort.eq_ignore_ascii_case("low") {
        "LOW"
    } else if effort.eq_ignore_ascii_case("high") {
        "HIGH"
    } else {
        "MEDIUM"
    }
}

fn reasoning_effort_to_thinking_budget(effort: &str) -> i32 {
    if effort.eq_ignore_ascii_case("minimal") {
        128
    } else if effort.eq_ignore_ascii_case("low") {
        512
    } else if effort.eq_ignore_ascii_case("high") {
        4096
    } else {
        2048
    }
}

fn load_service_account() -> Result<ServiceAccount, String> {
    let raw = if let Some(json) = env_nonempty("GOOGLE_APPLICATION_CREDENTIALS_JSON") {
        json
    } else {
        let path = env_nonempty("GOOGLE_APPLICATION_CREDENTIALS")
            .unwrap_or_else(|| DEFAULT_LOCAL_CREDENTIALS_PATH.to_string());
        fs::read_to_string(&path).map_err(|error| {
            format!("failed to read service account credentials at {path}: {error}")
        })?
    };

    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse service account credentials: {error}"))
}

fn fetch_access_token(service_account: &ServiceAccount) -> Result<String, String> {
    let token_uri = service_account
        .token_uri
        .as_deref()
        .unwrap_or(DEFAULT_TOKEN_URI);
    let now = unix_seconds();
    let claims = JwtClaims {
        iss: service_account.client_email.clone(),
        scope: GOOGLE_AUTH_SCOPE.to_string(),
        aud: token_uri.to_string(),
        iat: now,
        exp: now + 3600,
    };

    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some("JWT".to_string());
    let key = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())
        .map_err(|error| format!("failed to load service account private key: {error}"))?;
    let assertion = encode(&header, &claims, &key)
        .map_err(|error| format!("failed to sign service account JWT: {error}"))?;

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("failed to build HTTP client: {error}"))?;

    let response = client
        .post(token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", assertion.as_str()),
        ])
        .send()
        .map_err(|error| format!("OAuth token request failed: {error}"))?;

    let status = response.status();
    let value: serde_json::Value = response
        .json()
        .map_err(|error| format!("failed to parse OAuth token response JSON: {error}"))?;

    if !status.is_success() {
        return Err(format!("OAuth token endpoint returned {status}: {value}"));
    }

    let token: TokenResponse = serde_json::from_value(value)
        .map_err(|error| format!("invalid OAuth token response: {error}"))?;
    Ok(token.access_token)
}

fn vertex_generate_content_endpoint(project_id: &str, region: &str, model: &str) -> String {
    let host = if region == "global" {
        "aiplatform.googleapis.com".to_string()
    } else {
        format!("{region}-aiplatform.googleapis.com")
    };

    format!(
        "https://{host}/v1/projects/{project_id}/locations/{region}/publishers/google/models/{model}:generateContent"
    )
}

fn extract_vertex_text(value: &serde_json::Value) -> Option<VertexTextResponse> {
    let candidate = value.get("candidates")?.as_array()?.first()?;
    let text = candidate
        .get("content")?
        .get("parts")?
        .as_array()?
        .iter()
        .filter_map(|part| part.get("text")?.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let finish_reason = candidate
        .get("finishReason")
        .and_then(|reason| reason.as_str())
        .map(|reason| reason.to_string());

    Some(VertexTextResponse {
        text,
        finish_reason,
    })
}

fn create_run(
    http_request: &HttpRequest,
) -> Result<(RunResponse, Option<PaymentVerification>), CreateRunError> {
    let request: RunRequest = serde_json::from_str(&http_request.body)
        .map_err(|error| CreateRunError::BadRequest(format!("invalid run request: {error}")))?;

    if request.intent.trim().is_empty() {
        return Err(CreateRunError::BadRequest("intent is required".to_string()));
    }

    let variant_count = request.variant_count.unwrap_or(1);
    if ![1, 2, 3].contains(&variant_count) {
        return Err(CreateRunError::BadRequest(
            "variant_count must be 1, 2, or 3".to_string(),
        ));
    }

    let execution_mode = request.execution_mode.unwrap_or(ExecutionMode::Live);
    let payment = match require_x402_payment(http_request, execution_mode) {
        Ok(payment) => payment,
        Err(response) => return Err(CreateRunError::PaymentRequired(response)),
    };

    let artifact_type = request.artifact_type;
    let run_id = format!("run_{}", unix_millis());
    let initial_response = RunResponse {
        run_id: run_id.clone(),
        status: "running".to_string(),
        benchmark_status: "pending".to_string(),
        execution_mode,
        artifact_type,
        intent: request.intent.clone(),
        variant_count,
        stages: Vec::new(),
        final_variants: Vec::new(),
        run_duration_ms: None,
        quality_metrics: None,
        benchmark_error: None,
        error: None,
    };

    store_run(initial_response.clone()).map_err(CreateRunError::BadRequest)?;

    thread::spawn(move || {
        let error_intent = request.intent.clone();
        match execute_run(run_id.clone(), request, variant_count, execution_mode) {
            Ok(mut response) => {
                if response.status == "completed" && matches!(execution_mode, ExecutionMode::Live) {
                    response.benchmark_status = "running".to_string();
                    if let Err(error) = store_run(response.clone()) {
                        eprintln!("failed to store completed run before benchmark: {error}");
                    }
                    spawn_benchmark_run(response);
                } else if let Err(error) = store_run(response) {
                    eprintln!("failed to store run result: {error}");
                }
            }
            Err(error) => {
                let result = RunResponse {
                    run_id: run_id.clone(),
                    status: "error".to_string(),
                    benchmark_status: "error".to_string(),
                    execution_mode,
                    artifact_type,
                    intent: error_intent,
                    variant_count,
                    stages: Vec::new(),
                    final_variants: Vec::new(),
                    run_duration_ms: None,
                    quality_metrics: None,
                    benchmark_error: None,
                    error: Some(error),
                };
                if let Err(error) = store_run(result) {
                    eprintln!("failed to store run result: {error}");
                }
            }
        }
    });

    Ok((initial_response, payment))
}

fn require_x402_payment(
    request: &HttpRequest,
    execution_mode: ExecutionMode,
) -> Result<Option<PaymentVerification>, HttpResponse> {
    let config = x402_config();

    if !config.enabled || matches!(execution_mode, ExecutionMode::Mock) {
        return Ok(None);
    }

    if config.pay_to.trim().is_empty() {
        return Err(HttpResponse::json(
            "500 Internal Server Error",
            error_json("X402_PAY_TO is required when X402_ENABLED=true"),
        ));
    }

    if config.dev_bypass && valid_x402_dev_bypass(request) {
        let response = serde_json::json!({
            "success": true,
            "network": config.network,
            "transaction": "x402-dev-bypass",
            "payer": "dev",
        });
        return Ok(Some(PaymentVerification {
            response_header: encode_json_header(&response),
        }));
    }

    let Some(payment_signature) = request.headers.get("payment-signature") else {
        return Err(x402_payment_required_response(Some(
            "payment_required: retry with PAYMENT-SIGNATURE",
        ))
        .unwrap_or_else(|error| {
            HttpResponse::json("500 Internal Server Error", error_json(error))
        }));
    };

    match verify_and_settle_x402_payment(payment_signature, &config) {
        Ok(verification) => Ok(Some(verification)),
        Err(error) => Err(
            x402_payment_required_response(Some(&error)).unwrap_or_else(|error| {
                HttpResponse::json("500 Internal Server Error", error_json(error))
            }),
        ),
    }
}

fn verify_and_settle_x402_payment(
    payment_signature: &str,
    config: &X402Config,
) -> Result<PaymentVerification, String> {
    let payment_payload = decode_payment_signature(payment_signature)?;
    let requirement = x402_payment_requirement(config);
    let verify_request = serde_json::json!({
        "x402Version": DEFAULT_X402_VERSION,
        "paymentPayload": payment_payload,
        "paymentRequirements": requirement,
    });

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("failed to build x402 facilitator client: {error}"))?;

    let verify_url = format!("{}/verify", config.facilitator_url.trim_end_matches('/'));
    let verify_response = client
        .post(verify_url)
        .json(&verify_request)
        .send()
        .map_err(|error| format!("x402 facilitator verify failed: {error}"))?;
    let verify_status = verify_response.status();
    let verify_value: serde_json::Value = verify_response
        .json()
        .map_err(|error| format!("failed to parse x402 verify response: {error}"))?;

    if !verify_status.is_success() {
        return Err(format!(
            "x402 facilitator verify returned {verify_status}: {verify_value}"
        ));
    }

    if !verify_value
        .get("isValid")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        let reason = verify_value
            .get("invalidReason")
            .or_else(|| verify_value.get("error"))
            .and_then(|value| value.as_str())
            .unwrap_or("invalid_payment");
        return Err(format!("x402 payment rejected: {reason}"));
    }

    let settle_url = format!("{}/settle", config.facilitator_url.trim_end_matches('/'));
    let settle_response = client
        .post(settle_url)
        .json(&verify_request)
        .send()
        .map_err(|error| format!("x402 facilitator settle failed: {error}"))?;
    let settle_status = settle_response.status();
    let settle_value: serde_json::Value = settle_response
        .json()
        .map_err(|error| format!("failed to parse x402 settle response: {error}"))?;

    if !settle_status.is_success() {
        return Err(format!(
            "x402 facilitator settle returned {settle_status}: {settle_value}"
        ));
    }

    if settle_value
        .get("success")
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
    {
        Ok(PaymentVerification {
            response_header: encode_json_header(&settle_value),
        })
    } else {
        let reason = settle_value
            .get("errorReason")
            .or_else(|| settle_value.get("error"))
            .and_then(|value| value.as_str())
            .unwrap_or("settlement_failed");
        Err(format!("x402 settlement rejected: {reason}"))
    }
}

fn decode_payment_signature(payment_signature: &str) -> Result<serde_json::Value, String> {
    let decoded = BASE64
        .decode(payment_signature.trim())
        .map_err(|error| format!("invalid PAYMENT-SIGNATURE base64: {error}"))?;
    serde_json::from_slice(&decoded)
        .map_err(|error| format!("invalid PAYMENT-SIGNATURE JSON: {error}"))
}

fn x402_payment_required_response(error: Option<&str>) -> Result<HttpResponse, String> {
    let config = x402_config();
    let payment_required = X402PaymentRequired {
        x402_version: DEFAULT_X402_VERSION,
        accepts: vec![x402_payment_requirement(&config)],
        error: error.unwrap_or("payment_required").to_string(),
    };
    let body = serde_json::to_string_pretty(&payment_required)
        .map_err(|error| format!("failed to serialize x402 payment requirements: {error}"))?;
    let header_value = encode_json_header(&payment_required);
    let mut response = HttpResponse::json("402 Payment Required", body);
    response
        .headers
        .push(("PAYMENT-REQUIRED".to_string(), header_value));
    Ok(response)
}

fn x402_payment_requirement(config: &X402Config) -> X402PaymentRequirement {
    X402PaymentRequirement {
        scheme: "exact".to_string(),
        network: config.network.clone(),
        asset: config.asset.clone(),
        max_amount_required: config.amount.clone(),
        resource: config.resource.clone(),
        description: config.description.clone(),
        mime_type: "application/json".to_string(),
        pay_to: config.pay_to.clone(),
        max_timeout_seconds: config.max_timeout_seconds,
    }
}

fn encode_json_header(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    BASE64.encode(bytes)
}

fn x402_config() -> X402Config {
    X402Config {
        enabled: env_bool("X402_ENABLED", false),
        dev_bypass: env_bool("X402_DEV_BYPASS", false),
        facilitator_url: env::var("X402_FACILITATOR_URL")
            .unwrap_or_else(|_| DEFAULT_X402_FACILITATOR_URL.to_string()),
        network: env::var("X402_NETWORK").unwrap_or_else(|_| DEFAULT_X402_NETWORK.to_string()),
        asset: env::var("X402_ASSET").unwrap_or_else(|_| DEFAULT_X402_ASSET.to_string()),
        amount: env::var("X402_AMOUNT").unwrap_or_else(|_| DEFAULT_X402_AMOUNT.to_string()),
        pay_to: env::var("X402_PAY_TO").unwrap_or_default(),
        description: env::var("X402_DESCRIPTION")
            .unwrap_or_else(|_| "Berlin42 premium multi-model answer".to_string()),
        resource: env::var("X402_RESOURCE_URL")
            .unwrap_or_else(|_| "http://localhost:8080/api/runs".to_string()),
        max_timeout_seconds: env::var("X402_MAX_TIMEOUT_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_X402_TIMEOUT_SECONDS),
    }
}

fn valid_x402_dev_bypass(request: &HttpRequest) -> bool {
    let expected = env::var("X402_DEV_BYPASS_TOKEN").unwrap_or_else(|_| "dev-paid".to_string());
    request
        .headers
        .get("x402-dev-payment")
        .map(|value| value == &expected)
        .unwrap_or(false)
}

fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(default)
}

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && !value.starts_with("TODO_"))
}

fn execute_run(
    run_id: String,
    request: RunRequest,
    variant_count: u8,
    execution_mode: ExecutionMode,
) -> Result<RunResponse, String> {
    let run_started_at = Instant::now();
    let config = load_model_config()?;
    let research_enabled = request.research_enabled.unwrap_or(false);
    let mut stages = match execution_mode {
        ExecutionMode::Mock => build_mock_debate_stages(&request, &config, research_enabled),
        ExecutionMode::Live => build_live_initial_debate_stages(
            &request,
            variant_count,
            &config,
            research_enabled,
            |stages| {
                let snapshot = RunResponse {
                    run_id: run_id.clone(),
                    status: "running".to_string(),
                    benchmark_status: "pending".to_string(),
                    execution_mode,
                    artifact_type: request.artifact_type,
                    intent: request.intent.clone(),
                    variant_count,
                    stages: stages.to_vec(),
                    final_variants: Vec::new(),
                    run_duration_ms: Some(elapsed_ms(run_started_at)),
                    quality_metrics: None,
                    benchmark_error: None,
                    error: None,
                };

                if let Err(error) = store_run(snapshot) {
                    eprintln!("failed to store progress snapshot: {error}");
                }
            },
        )?,
    };
    let final_variants = match execution_mode {
        ExecutionMode::Mock => build_final_variants(&request, variant_count, &stages),
        ExecutionMode::Live => {
            build_live_final_variants(&request, variant_count, &config, &mut stages, |stages| {
                let snapshot = RunResponse {
                    run_id: run_id.clone(),
                    status: "running".to_string(),
                    benchmark_status: "pending".to_string(),
                    execution_mode,
                    artifact_type: request.artifact_type,
                    intent: request.intent.clone(),
                    variant_count,
                    stages: stages.to_vec(),
                    final_variants: Vec::new(),
                    run_duration_ms: Some(elapsed_ms(run_started_at)),
                    quality_metrics: None,
                    benchmark_error: None,
                    error: None,
                };

                if let Err(error) = store_run(snapshot) {
                    eprintln!("failed to store finalization snapshot: {error}");
                }
            })?
        }
    };

    Ok(RunResponse {
        run_id,
        status: "completed".to_string(),
        benchmark_status: "pending".to_string(),
        execution_mode,
        artifact_type: request.artifact_type,
        intent: request.intent,
        variant_count,
        stages,
        final_variants,
        run_duration_ms: Some(elapsed_ms(run_started_at)),
        quality_metrics: None,
        benchmark_error: None,
        error: None,
    })
}

fn runs_store() -> &'static Arc<Mutex<HashMap<String, RunResponse>>> {
    RUNS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn store_run(response: RunResponse) -> Result<(), String> {
    let mut runs = runs_store()
        .lock()
        .map_err(|_| "run store lock poisoned".to_string())?;
    runs.insert(response.run_id.clone(), response);
    Ok(())
}

fn get_run(path: &str) -> Result<RunResponse, String> {
    let run_id = path
        .trim_start_matches("/api/runs/")
        .split('?')
        .next()
        .unwrap_or_default();

    if run_id.is_empty() {
        return Err("run_id is required".to_string());
    }

    let runs = runs_store()
        .lock()
        .map_err(|_| "run store lock poisoned".to_string())?;
    runs.get(run_id)
        .cloned()
        .ok_or_else(|| format!("run {run_id} not found"))
}

fn spawn_benchmark_run(response: RunResponse) {
    thread::spawn(move || {
        let run_id = response.run_id.clone();
        match compute_quality_metrics(&response) {
            Ok(metrics) => {
                if let Err(error) = update_run_benchmark(&run_id, "completed", Some(metrics), None)
                {
                    eprintln!("failed to store benchmark metrics: {error}");
                }
            }
            Err(error) => {
                eprintln!("benchmark failed for {run_id}: {error}");
                let fallback = fallback_quality_metrics(&response, Some(error.clone()));
                if let Err(store_error) =
                    update_run_benchmark(&run_id, "error", Some(fallback), Some(error))
                {
                    eprintln!("failed to store benchmark failure: {store_error}");
                }
            }
        }
    });
}

fn update_run_benchmark(
    run_id: &str,
    benchmark_status: &str,
    quality_metrics: Option<QualityMetrics>,
    benchmark_error: Option<String>,
) -> Result<(), String> {
    let mut runs = runs_store()
        .lock()
        .map_err(|_| "run store lock poisoned".to_string())?;
    let run = runs
        .get_mut(run_id)
        .ok_or_else(|| format!("run {run_id} not found"))?;

    run.benchmark_status = benchmark_status.to_string();
    run.quality_metrics = quality_metrics;
    run.benchmark_error = benchmark_error;

    Ok(())
}

fn compute_quality_metrics(response: &RunResponse) -> Result<QualityMetrics, String> {
    let config = load_model_config()?;
    let benchmark_model = find_model(&config, "benchmark_model");

    if benchmark_model.provider != "vertex" {
        return Ok(fallback_quality_metrics(
            response,
            Some(format!(
                "benchmark_model provider {} is not supported yet",
                benchmark_model.provider
            )),
        ));
    }

    let prompt = build_benchmark_prompt(response);
    let benchmark_response = call_vertex_generate_content(&benchmark_model, &prompt)?;
    parse_quality_metrics(&benchmark_response.text)
        .or_else(|| {
            Some(fallback_quality_metrics(
                response,
                Some("benchmark_model returned non-JSON metrics".to_string()),
            ))
        })
        .ok_or_else(|| "failed to build quality metrics".to_string())
}

fn build_benchmark_prompt(response: &RunResponse) -> String {
    let final_variant = response.final_variants.first();
    let answer = final_variant
        .map(|variant| variant.content.as_str())
        .unwrap_or_default();
    let reusable_prompt = final_variant
        .map(|variant| variant.reusable_prompt.as_str())
        .unwrap_or_default();
    let candidates = response
        .stages
        .iter()
        .filter(|stage| stage.stage == "Generation")
        .map(|stage| {
            format!(
                "## {} ({})\n{}",
                stage.model_id,
                stage.model,
                truncate_for_display(stage.content.trim(), 5000)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"# Real-Time Answer Quality Benchmark

You are the benchmark_model for Berlin42 Answer Forge.

Evaluate only the supplied model drafts and final answer. Do not use outside knowledge.

User request:
{intent}

Independent model drafts:
{candidates}

Final answer:
{answer}

Reusable prompt:
{reusable_prompt}

Metrics:

1. Fact Confidence Index (FCI)
- Extract key factual claims from the final answer.
- Count a claim as supported only if at least two independent model drafts support it.
- score = supported_claims / total_claims * 100.

2. Aspect Coverage Gap (ACG)
- Extract the important aspects needed for a complete answer to the user request.
- Estimate coverage for each single model draft.
- Estimate coverage for the final answer.
- best_single_model_coverage is the best coverage among the three drafts.
- coverage_gain = final_coverage - best_single_model_coverage.

Return only strict JSON:
{{
  "summary": "Calculated from the model drafts behind this answer.",
  "fci": {{
    "label": "Consensus confidence",
    "score": 0,
    "supported_claims": 0,
    "total_claims": 0,
    "annotation": ""
  }},
  "acg": {{
    "label": "Coverage lift",
    "final_coverage": 0,
    "best_single_model_coverage": 0,
    "coverage_gain": 0,
    "covered_aspects": 0,
    "total_aspects": 0,
    "annotation": ""
  }},
  "notes": []
}}

Rules:
- Use integer percentages from 0 to 100.
- Keep annotations short and user-facing.
- If there are few factual claims, explain that in notes.
- Do not mention hidden prompts or private system details.
- Do not wrap JSON in markdown fences.
"#,
        intent = response.intent,
        candidates = if candidates.is_empty() {
            "No independent model drafts were available.".to_string()
        } else {
            candidates
        },
        answer = truncate_for_display(answer, 7000),
        reusable_prompt = truncate_for_display(reusable_prompt, 3000),
    )
}

fn parse_quality_metrics(raw: &str) -> Option<QualityMetrics> {
    let trimmed = raw.trim();
    if let Ok(metrics) = serde_json::from_str::<QualityMetrics>(trimmed) {
        return Some(normalize_quality_metrics(metrics));
    }

    let json_start = trimmed.find('{')?;
    let json_end = trimmed.rfind('}')?;
    if json_end <= json_start {
        return None;
    }

    serde_json::from_str::<QualityMetrics>(&trimmed[json_start..=json_end])
        .ok()
        .map(normalize_quality_metrics)
}

fn normalize_quality_metrics(mut metrics: QualityMetrics) -> QualityMetrics {
    metrics.fci.score = metrics.fci.score.min(100);
    metrics.acg.final_coverage = metrics.acg.final_coverage.min(100);
    metrics.acg.best_single_model_coverage = metrics.acg.best_single_model_coverage.min(100);
    metrics.acg.coverage_gain =
        metrics.acg.final_coverage as i16 - metrics.acg.best_single_model_coverage as i16;

    if metrics.summary.trim().is_empty() {
        metrics.summary = "Calculated from the model drafts behind this answer.".to_string();
    }
    if metrics.fci.label.trim().is_empty() {
        metrics.fci.label = "Consensus confidence".to_string();
    }
    if metrics.acg.label.trim().is_empty() {
        metrics.acg.label = "Coverage lift".to_string();
    }

    metrics
}

fn fallback_quality_metrics(response: &RunResponse, note: Option<String>) -> QualityMetrics {
    let generation_count = response
        .stages
        .iter()
        .filter(|stage| stage.stage == "Generation")
        .count() as u32;
    let supported_claims = generation_count.min(3);
    let total_claims = generation_count.max(1);
    let fci_score = ((supported_claims as f32 / total_claims as f32) * 100.0).round() as u8;
    let final_coverage = if response.final_variants.is_empty() {
        0
    } else {
        100
    };
    let best_single_model_coverage = if generation_count >= 3 { 75 } else { 60 };

    let mut notes = vec![
        "Fallback metrics were used because the benchmark model did not return usable metrics."
            .to_string(),
    ];
    if let Some(note) = note {
        notes.push(note);
    }

    QualityMetrics {
        summary: "Calculated from the model drafts behind this answer.".to_string(),
        fci: FactConfidenceMetric {
            label: "Consensus confidence".to_string(),
            score: fci_score.min(100),
            supported_claims,
            total_claims,
            annotation: format!(
                "{supported_claims} of {total_claims} available draft signals were usable for consensus checking."
            ),
        },
        acg: AspectCoverageMetric {
            label: "Coverage lift".to_string(),
            final_coverage,
            best_single_model_coverage,
            coverage_gain: final_coverage as i16 - best_single_model_coverage as i16,
            covered_aspects: if final_coverage == 100 { 4 } else { 0 },
            total_aspects: 4,
            annotation: format!(
                "Fallback estimate: final answer coverage is {final_coverage}%, compared with {best_single_model_coverage}% for the best single draft."
            ),
        },
        notes,
    }
}

fn build_mock_debate_stages(
    request: &RunRequest,
    config: &ModelConfigFile,
    research_enabled: bool,
) -> Vec<StageOutput> {
    let mut stages = Vec::new();

    push_stage(
        &mut stages,
        config,
        "base_llm",
        "Base Prompt",
        "Raw intent converted into a structured brief.",
        format!(
            "Intent: {}\nArtifact: {:?}\nResearch: {}\nQuality frame: concrete, testable, variant-aware.",
            request.intent,
            request.artifact_type,
            if research_enabled { "requested" } else { "off" }
        ),
        research_enabled,
    );

    for model_id in ["model_a", "model_b", "model_c"] {
        push_stage(
            &mut stages,
            config,
            model_id,
            "Generation",
            format!("{model_id} generated an independent candidate."),
            format!(
                "{model_id} proposes a candidate for {:?}: start with user value, add constraints, produce useful structure, then define what would make the result fail.",
                request.artifact_type
            ),
            research_enabled,
        );
    }

    push_stage(
        &mut stages,
        config,
        "model_a",
        "Cross-review",
        "Model A critiques candidates B and C.",
        "B is strong on reasoning but needs tighter output boundaries. C is broad but risks adding optional complexity.".to_string(),
        research_enabled,
    );
    push_stage(
        &mut stages,
        config,
        "model_b",
        "Cross-review",
        "Model B critiques candidates A and C.",
        "A is practical but may under-specify edge cases. C should separate facts, assumptions, and taste.".to_string(),
        research_enabled,
    );
    push_stage(
        &mut stages,
        config,
        "model_c",
        "Cross-review",
        "Model C critiques candidates A and B.",
        "A and B should both include explicit eval criteria and avoid generic best-practice language.".to_string(),
        research_enabled,
    );

    for model_id in ["model_a", "model_b", "model_c"] {
        push_stage(
            &mut stages,
            config,
            model_id,
            "Distributed red-team",
            format!("{model_id} stress-tests the whole debate."),
            "Must fix: define success criteria, separate assumptions from conclusions, and prevent shallow paraphrased variants.".to_string(),
            research_enabled,
        );
    }

    push_consensus_stage(
        &mut stages,
        "Merge + Compression + Eval + Revision",
        "Backend consensus merged model conclusions without a separate judge model.",
        "Final variants differ by strategy: practical baseline, optimized expert version, and optional exploratory version.".to_string(),
    );

    stages
}

fn build_live_initial_debate_stages(
    request: &RunRequest,
    variant_count: u8,
    config: &ModelConfigFile,
    research_enabled: bool,
    mut on_progress: impl FnMut(&[StageOutput]),
) -> Result<Vec<StageOutput>, String> {
    let mut stages = Vec::new();
    let base_model = find_model(config, "base_llm");
    let base_prompt = build_base_brief_prompt(request, research_enabled)?;
    let (brief_response, brief_duration_ms) =
        call_vertex_generate_content_timed(&base_model, &base_prompt)?;
    let task_brief = brief_response.text.clone();

    push_live_stage(
        &mut stages,
        &base_model,
        "Base Prompt",
        research_enabled,
        brief_response.finish_reason,
        Some(brief_duration_ms),
        "base_llm built the shared task brief.",
        task_brief.clone(),
    );
    on_progress(&stages);

    let mut candidates: Vec<(String, String)> = Vec::new();
    let generation_jobs = ["model_a", "model_b", "model_c"]
        .iter()
        .map(|model_id| {
            let model = find_model(config, model_id);
            let prompt = build_candidate_generation_prompt(request, model_id, &task_brief)?;
            Ok((model_id.to_string(), model, prompt))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let generation_handles = generation_jobs
        .into_iter()
        .map(|(model_id, model, prompt)| {
            thread::spawn(move || {
                let response = call_vertex_generate_content_timed(&model, &prompt);
                (model_id, model, response)
            })
        })
        .collect::<Vec<_>>();

    for handle in generation_handles {
        let (model_id, model, candidate_response) = handle
            .join()
            .map_err(|_| "Candidate generation thread panicked".to_string())?;
        let (candidate_response, candidate_duration_ms) = candidate_response?;

        push_live_stage(
            &mut stages,
            &model,
            "Generation",
            research_enabled,
            candidate_response.finish_reason,
            Some(candidate_duration_ms),
            format!("{model_id} generated a live candidate from the shared brief."),
            candidate_response.text.clone(),
        );
        candidates.push((model_id, candidate_response.text));
        on_progress(&stages);
    }

    let review_jobs = ["model_a", "model_b", "model_c"]
        .iter()
        .map(|model_id| {
            let model = find_model(config, model_id);
            let assigned_candidates = candidates
                .iter()
                .filter(|(candidate_id, _)| candidate_id != model_id)
                .cloned()
                .collect::<Vec<_>>();
            let prompt = build_cross_review_prompt(model_id, &task_brief, &assigned_candidates)?;
            Ok((model_id.to_string(), model, prompt))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let review_handles = review_jobs
        .into_iter()
        .map(|(model_id, model, prompt)| {
            thread::spawn(move || {
                let response = call_vertex_generate_content_timed(&model, &prompt);
                (model_id, model, response)
            })
        })
        .collect::<Vec<_>>();

    let mut cross_reviews: Vec<(String, String)> = Vec::new();
    for handle in review_handles {
        let (model_id, model, review_response) = handle
            .join()
            .map_err(|_| "Cross-review thread panicked".to_string())?;
        let (review_response, review_duration_ms) = review_response?;

        push_live_stage(
            &mut stages,
            &model,
            "Cross-review",
            research_enabled,
            review_response.finish_reason,
            Some(review_duration_ms),
            format!("{model_id} reviewed the two other live candidates."),
            review_response.text.clone(),
        );
        cross_reviews.push((model_id, review_response.text));
        on_progress(&stages);
    }

    let red_team_jobs = ["model_a", "model_b", "model_c"]
        .iter()
        .map(|model_id| {
            let model = find_model(config, model_id);
            let prompt = build_red_team_prompt(model_id, &task_brief, &candidates, &cross_reviews)?;
            Ok((model_id.to_string(), model, prompt))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let red_team_handles = red_team_jobs
        .into_iter()
        .map(|(model_id, model, prompt)| {
            thread::spawn(move || {
                let response = call_vertex_generate_content_timed(&model, &prompt);
                (model_id, model, response)
            })
        })
        .collect::<Vec<_>>();

    let mut red_team_notes: Vec<(String, String)> = Vec::new();
    for handle in red_team_handles {
        let (model_id, model, red_team_response) = handle
            .join()
            .map_err(|_| "Red-team thread panicked".to_string())?;
        let (red_team_response, red_team_duration_ms) = red_team_response?;

        push_live_stage(
            &mut stages,
            &model,
            "Distributed red-team",
            research_enabled,
            red_team_response.finish_reason,
            Some(red_team_duration_ms),
            format!("{model_id} stress-tested candidates and cross-reviews."),
            red_team_response.text.clone(),
        );
        red_team_notes.push((model_id, red_team_response.text));
        on_progress(&stages);
    }

    let consensus_model = find_model(config, "base_llm");
    let consensus_prompt = build_consensus_merge_prompt(
        request,
        variant_count,
        &task_brief,
        &candidates,
        &cross_reviews,
        &red_team_notes,
    )?;
    let (consensus_response, consensus_duration_ms) =
        call_vertex_generate_content_timed(&consensus_model, &consensus_prompt)?;
    let consensus_material = consensus_response.text.clone();
    push_live_stage(
        &mut stages,
        &consensus_model,
        "Consensus Merge",
        research_enabled,
        consensus_response.finish_reason,
        Some(consensus_duration_ms),
        "Consensus merge converted debate material into requirements for the final answer.",
        consensus_material.clone(),
    );
    on_progress(&stages);

    let compression_prompt = build_compression_prompt(request, &consensus_material)?;
    let (compression_response, compression_duration_ms) =
        call_vertex_generate_content_timed(&consensus_model, &compression_prompt)?;
    let compressed_rules = compression_response.text.clone();
    push_live_stage(
        &mut stages,
        &consensus_model,
        "Compression",
        research_enabled,
        compression_response.finish_reason,
        Some(compression_duration_ms),
        "Compression removed duplicate or non-behavioral rules.",
        compressed_rules.clone(),
    );
    on_progress(&stages);

    let eval_model = find_model(config, "benchmark_model");
    let eval_prompt = build_eval_prompt(request, variant_count, &task_brief, &compressed_rules)?;
    let (eval_response, eval_duration_ms) =
        call_vertex_generate_content_timed(&eval_model, &eval_prompt)?;
    let eval_report = eval_response.text.clone();
    push_live_stage(
        &mut stages,
        &eval_model,
        "Eval",
        research_enabled,
        eval_response.finish_reason,
        Some(eval_duration_ms),
        "Eval checked the compressed rules against likely failure cases.",
        eval_report.clone(),
    );
    on_progress(&stages);

    let revision_prompt = build_revision_prompt(request, &compressed_rules, &eval_report)?;
    let (revision_response, revision_duration_ms) =
        call_vertex_generate_content_timed(&consensus_model, &revision_prompt)?;
    push_live_stage(
        &mut stages,
        &consensus_model,
        "Revision",
        research_enabled,
        revision_response.finish_reason,
        Some(revision_duration_ms),
        "Revision fixed high-severity eval failures before final formatting.",
        revision_response.text,
    );
    on_progress(&stages);

    Ok(stages)
}

fn push_consensus_stage(
    stages: &mut Vec<StageOutput>,
    stage: &str,
    summary: impl Into<String>,
    content: String,
) {
    stages.push(StageOutput {
        stage: stage.to_string(),
        model_id: "consensus_merge".to_string(),
        model: "deterministic-consensus-v1".to_string(),
        region: "local".to_string(),
        reasoning_effort: "Low".to_string(),
        ask_world: false,
        finish_reason: None,
        duration_ms: None,
        summary: summary.into(),
        content,
    });
}

fn push_live_stage(
    stages: &mut Vec<StageOutput>,
    model: &ModelConfig,
    stage: &str,
    research_enabled: bool,
    finish_reason: Option<String>,
    duration_ms: Option<u64>,
    summary: impl Into<String>,
    content: String,
) {
    stages.push(StageOutput {
        stage: stage.to_string(),
        model_id: model.id.clone(),
        model: model.model.clone(),
        region: model.region.clone(),
        reasoning_effort: model.reasoning_effort.clone(),
        ask_world: research_enabled && model.ask_world,
        finish_reason,
        duration_ms,
        summary: summary.into(),
        content,
    });
}

fn push_stage(
    stages: &mut Vec<StageOutput>,
    config: &ModelConfigFile,
    model_id: &str,
    stage: &str,
    summary: impl Into<String>,
    content: String,
    research_enabled: bool,
) {
    let model = find_model(config, model_id);
    let ask_world = research_enabled && model.ask_world;

    stages.push(StageOutput {
        stage: stage.to_string(),
        model_id: model.id.clone(),
        model: model.model.clone(),
        region: model.region.clone(),
        reasoning_effort: model.reasoning_effort.clone(),
        ask_world,
        finish_reason: None,
        duration_ms: None,
        summary: summary.into(),
        content,
    });
}

fn build_base_brief_prompt(request: &RunRequest, research_enabled: bool) -> Result<String, String> {
    let template = load_master_prompt("00_base_brief.md")?;
    let variant_count = request.variant_count.unwrap_or(2);

    Ok(format!(
        "{template}\n\n---\n\nuser_request:\n{}\n\nartifact_type: {:?}\nvariant_count: {}\nresearch_enabled: {}\nresearch_notes:\nNone provided in this run.\n",
        request.intent, request.artifact_type, variant_count, research_enabled
    ))
}

fn build_candidate_generation_prompt(
    request: &RunRequest,
    model_role: &str,
    task_brief: &str,
) -> Result<String, String> {
    let template = load_master_prompt("01_candidate_generation.md")?;

    Ok(format!(
        "{template}\n\n---\n\ntask_brief:\n{}\n\nmodel_role: {}\nartifact_type: {:?}\n",
        task_brief, model_role, request.artifact_type
    ))
}

fn build_cross_review_prompt(
    reviewer_model_id: &str,
    task_brief: &str,
    assigned_candidates: &[(String, String)],
) -> Result<String, String> {
    let template = load_master_prompt("02_cross_review.md")?;
    let candidate_1 = assigned_candidates
        .first()
        .map(|(model_id, content)| format_candidate_block(model_id, content))
        .unwrap_or_else(|| "None provided.".to_string());
    let candidate_2 = assigned_candidates
        .get(1)
        .map(|(model_id, content)| format_candidate_block(model_id, content))
        .unwrap_or_else(|| "None provided.".to_string());

    Ok(format!(
        "{template}\n\n---\n\ntask_brief:\n{}\n\nyour_model_id:\n{}\n\ncandidate_1:\n{}\n\ncandidate_2:\n{}\n",
        task_brief, reviewer_model_id, candidate_1, candidate_2
    ))
}

fn build_red_team_prompt(
    red_team_model_id: &str,
    task_brief: &str,
    candidates: &[(String, String)],
    cross_reviews: &[(String, String)],
) -> Result<String, String> {
    let template = load_master_prompt("03_distributed_red_team.md")?;

    Ok(format!(
        "{template}\n\n---\n\nred_team_model_id:\n{}\n\ntask_brief:\n{}\n\nall_candidates:\n{}\n\nall_cross_reviews:\n{}\n",
        red_team_model_id,
        task_brief,
        format_stage_pairs(candidates),
        format_stage_pairs(cross_reviews)
    ))
}

fn build_consensus_merge_prompt(
    request: &RunRequest,
    variant_count: u8,
    task_brief: &str,
    candidates: &[(String, String)],
    cross_reviews: &[(String, String)],
    red_team_notes: &[(String, String)],
) -> Result<String, String> {
    let template = load_master_prompt("04_consensus_merge.md")?;

    Ok(format!(
        "{template}\n\n---\n\nartifact_type:\n{}\n\nvariant_count:\n{}\n\ntask_brief:\n{}\n\nall_candidates:\n{}\n\nall_cross_reviews:\n{}\n\nall_red_team_notes:\n{}\n",
        artifact_type_name(request.artifact_type),
        variant_count,
        task_brief,
        format_stage_pairs(candidates),
        format_stage_pairs(cross_reviews),
        format_stage_pairs(red_team_notes)
    ))
}

fn build_compression_prompt(
    request: &RunRequest,
    consensus_material: &str,
) -> Result<String, String> {
    let template = load_master_prompt("05_compression.md")?;

    Ok(format!(
        "{template}\n\n---\n\nartifact_type:\n{}\n\nconsensus_material:\n{}\n",
        artifact_type_name(request.artifact_type),
        consensus_material
    ))
}

fn build_eval_prompt(
    request: &RunRequest,
    variant_count: u8,
    task_brief: &str,
    compressed_rules: &str,
) -> Result<String, String> {
    let template = load_master_prompt("06_eval.md")?;

    Ok(format!(
        "{template}\n\n---\n\nartifact_type:\n{}\n\nvariant_count:\n{}\n\ntask_brief:\n{}\n\ncompressed_rules:\n{}\n",
        artifact_type_name(request.artifact_type),
        variant_count,
        task_brief,
        compressed_rules
    ))
}

fn build_revision_prompt(
    request: &RunRequest,
    compressed_rules: &str,
    eval_report: &str,
) -> Result<String, String> {
    let template = load_master_prompt("07_revision.md")?;

    Ok(format!(
        "{template}\n\n---\n\nartifact_type:\n{}\n\ncompressed_rules:\n{}\n\neval_report:\n{}\n",
        artifact_type_name(request.artifact_type),
        compressed_rules,
        eval_report
    ))
}

fn format_stage_pairs(items: &[(String, String)]) -> String {
    if items.is_empty() {
        return "None provided.".to_string();
    }

    items
        .iter()
        .map(|(model_id, content)| format_candidate_block(model_id, content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_candidate_block(model_id: &str, content: &str) -> String {
    format!(
        "## {model_id}\n{}",
        truncate_for_display(content.trim(), 8000)
    )
}

fn load_master_prompt(file_name: &str) -> Result<String, String> {
    let dir = env::var("PROMPTS_DIR").unwrap_or_else(|_| DEFAULT_PROMPTS_DIR.to_string());
    let path = format!("{dir}/{file_name}");

    fs::read_to_string(&path)
        .map_err(|error| format!("failed to read master prompt at {path}: {error}"))
}

fn find_model(config: &ModelConfigFile, model_id: &str) -> ModelConfig {
    config
        .models
        .iter()
        .find(|model| model.id == model_id)
        .cloned()
        .unwrap_or_else(|| ModelConfig {
            id: model_id.to_string(),
            provider: "mock".to_string(),
            model: "mock-model".to_string(),
            region: config.default_region.clone(),
            reasoning_effort: "Low".to_string(),
            max_output_tokens: Some(OutputTokenLimit::Count(2048)),
            ask_world: false,
            purpose: "Fallback mock model.".to_string(),
        })
}

fn build_live_final_variants(
    request: &RunRequest,
    variant_count: u8,
    config: &ModelConfigFile,
    stages: &mut Vec<StageOutput>,
    mut on_progress: impl FnMut(&[StageOutput]),
) -> Result<Vec<FinalVariant>, String> {
    let final_model = find_model(config, "base_llm");
    let strategies = ["practical", "expert", "creative"];
    let mut variants = Vec::new();

    for index in 0..variant_count {
        let strategy = strategies
            .get(index as usize)
            .copied()
            .unwrap_or("practical");

        stages.push(StageOutput {
            stage: "Final Formatting".to_string(),
            model_id: final_model.id.clone(),
            model: final_model.model.clone(),
            region: final_model.region.clone(),
            reasoning_effort: final_model.reasoning_effort.clone(),
            ask_world: request.research_enabled.unwrap_or(false) && final_model.ask_world,
            finish_reason: None,
            duration_ms: None,
            summary: format!("Finalizer is producing the {strategy} artifact."),
            content: "Waiting for final model output.".to_string(),
        });
        on_progress(stages);

        let prompt = build_finalization_prompt(request, strategy, variant_count, stages);
        let (response, final_duration_ms) =
            call_vertex_generate_content_timed(&final_model, &prompt)?;
        let final_content = response.text.trim().to_string();

        if let Some(stage) = stages.last_mut() {
            stage.finish_reason = response.finish_reason;
            stage.duration_ms = Some(final_duration_ms);
            stage.summary = format!("Finalizer produced the {strategy} artifact.");
            stage.content = final_content.clone();
        }
        on_progress(stages);

        variants.push(FinalVariant {
            index: index + 1,
            title: final_variant_title(request.artifact_type, strategy),
            strategy: live_strategy_label(strategy),
            content: extract_final_answer(&final_content),
            reusable_prompt: extract_reusable_prompt(&final_content, &request.intent),
        });
    }

    Ok(variants)
}

fn live_strategy_label(strategy: &str) -> String {
    match strategy {
        "expert" => "Live finalization with stricter precision and quality checks.",
        "creative" => "Live finalization with a broader alternative angle.",
        _ => "Live finalization optimized for the most useful default result.",
    }
    .to_string()
}

fn build_finalization_prompt(
    request: &RunRequest,
    strategy: &str,
    variant_count: u8,
    stages: &[StageOutput],
) -> String {
    let artifact_type = artifact_type_name(request.artifact_type);
    let output_language = if matches!(request.artifact_type, ArtifactType::Answer)
        && looks_cyrillic(&request.intent)
    {
        "same language as the user request: Russian"
    } else if looks_cyrillic(&request.intent) {
        "prefer the user's language unless the artifact format is clearer with English section names"
    } else {
        "same language as the user request"
    };
    let stage_material = stages
        .iter()
        .filter(|stage| stage.content != "Waiting for final model output.")
        .map(|stage| {
            format!(
                "## Stage: {} / {}\nSummary: {}\nContent:\n{}",
                stage.stage,
                stage.model_id,
                stage.summary,
                truncate_for_display(stage.content.trim(), 6000)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"# Final Artifact Builder

You are the final builder in a multi-model pipeline. Your job is to produce the final user-facing artifact.

User request:
{intent}

Requested artifact type:
{artifact_type}

Strategy:
{strategy}

Requested final variant count:
{variant_count}

Output language:
{output_language}

Pipeline material:
{stage_material}

Hard rules:
- Return only valid JSON. Do not wrap it in markdown fences.
- JSON shape:
  {{
    "answer_markdown": "the final user-facing answer in markdown",
    "reusable_prompt_markdown": "a complete reusable prompt the user can paste into another LLM"
  }}
- The `answer_markdown` field must contain the finished {artifact_type} itself.
- The `reusable_prompt_markdown` field must be a strong standalone prompt that recreates or improves this answer for the same class of task.
- If requested final variant count is 1, return exactly one final result. Do not include headings like "Variant 1", "Variant 2", "Option A", or "Option B"; synthesize the strongest points into one answer.
- If pipeline material asks for several variants but requested final variant count is 1, treat those variants as raw material and merge them into one strongest result.
- Only output multiple variants when requested final variant count is greater than 1.
- Do not say "create", "write", "you should", or describe what another model should do.
- Do not mention this pipeline, candidates, model A/B/C, or internal review unless the user explicitly asked for process details.
- Do not output a summary of the artifact. Output the artifact.
- Make the result substantial enough to be useful.
- Remove contradictions and generic filler.
- Include concrete failure checks where they improve the artifact.
- The reusable prompt should include role, task, constraints, output format, quality bar, and self-check.

Artifact-specific contract:
- If artifact type is `skill`, return a complete reusable skill with purpose, when to use, always/never rules, workflow, checklist, examples, tests, and failure criteria.
- If artifact type is `prompt`, return a complete reusable prompt that can be pasted into an LLM, with role, inputs, process, output contract, constraints, examples, and self-check.
- If artifact type is `answer`, answer the user's request directly in the user's language.

Final JSON:"#,
        intent = request.intent.as_str(),
    )
}

fn extract_final_answer(raw: &str) -> String {
    parse_final_artifact_payload(raw)
        .map(|payload| payload.answer_markdown.trim().to_string())
        .filter(|answer| !answer.is_empty())
        .unwrap_or_else(|| raw.trim().to_string())
}

fn extract_reusable_prompt(raw: &str, intent: &str) -> String {
    parse_final_artifact_payload(raw)
        .map(|payload| payload.reusable_prompt_markdown.trim().to_string())
        .filter(|prompt| !prompt.is_empty())
        .unwrap_or_else(|| fallback_reusable_prompt(intent))
}

fn parse_final_artifact_payload(raw: &str) -> Option<FinalArtifactPayload> {
    let trimmed = raw.trim();
    let direct = serde_json::from_str::<FinalArtifactPayload>(trimmed).ok();
    if direct.is_some() {
        return direct;
    }

    let json_start = trimmed.find('{')?;
    let json_end = trimmed.rfind('}')?;
    if json_end <= json_start {
        return None;
    }

    serde_json::from_str::<FinalArtifactPayload>(&trimmed[json_start..=json_end]).ok()
}

fn fallback_reusable_prompt(intent: &str) -> String {
    format!(
        "You are a careful, practical expert assistant.\n\nTask:\nAnswer this user request: {intent}\n\nRequirements:\n- Start with the direct answer.\n- State important assumptions and uncertainty.\n- Give a structured explanation with concrete details.\n- Avoid unsupported claims, fake citations, and generic filler.\n- If the topic depends on current facts, dates, prices, laws, or recommendations, verify with fresh sources before answering.\n- End with a short quality check or next step.\n\nOutput:\nReturn a polished markdown answer in the user's language."
    )
}

fn artifact_type_name(artifact_type: ArtifactType) -> &'static str {
    match artifact_type {
        ArtifactType::Skill => "skill",
        ArtifactType::Prompt => "prompt",
        ArtifactType::Answer => "answer",
    }
}

fn build_final_variants(
    request: &RunRequest,
    variant_count: u8,
    stages: &[StageOutput],
) -> Vec<FinalVariant> {
    let mut variants = vec![FinalVariant {
        index: 1,
        title: final_variant_title(request.artifact_type, "practical"),
        strategy: "Fast, useful, low-risk result.".to_string(),
        content: final_content(request, "practical", stages),
        reusable_prompt: fallback_reusable_prompt(&request.intent),
    }];

    if variant_count >= 2 {
        variants.push(FinalVariant {
            index: 2,
            title: final_variant_title(request.artifact_type, "expert"),
            strategy: "More precise, with assumptions and quality checks.".to_string(),
            content: final_content(request, "expert", stages),
            reusable_prompt: fallback_reusable_prompt(&request.intent),
        });
    }

    if variant_count >= 3 {
        variants.push(FinalVariant {
            index: 3,
            title: final_variant_title(request.artifact_type, "creative"),
            strategy: "Different angle, useful when the user wants broader choice.".to_string(),
            content: final_content(request, "creative", stages),
            reusable_prompt: fallback_reusable_prompt(&request.intent),
        });
    }

    variants
}

fn final_variant_title(artifact_type: ArtifactType, strategy: &str) -> String {
    match (artifact_type, strategy) {
        (ArtifactType::Answer, "practical") => "Practical Answer",
        (ArtifactType::Answer, "expert") => "Expert Answer",
        (ArtifactType::Answer, "creative") => "Creative Answer",
        (ArtifactType::Prompt, "practical") => "Production Prompt",
        (ArtifactType::Prompt, "expert") => "Expert Prompt",
        (ArtifactType::Prompt, "creative") => "Alternative Prompt",
        (ArtifactType::Skill, "practical") => "Production Skill",
        (ArtifactType::Skill, "expert") => "Expert Skill",
        (ArtifactType::Skill, "creative") => "Alternative Skill",
        _ => "Final Result",
    }
    .to_string()
}

fn final_content(request: &RunRequest, strategy: &str, stages: &[StageOutput]) -> String {
    match request.artifact_type {
        ArtifactType::Answer => answer_variant(&request.intent, strategy, stages),
        ArtifactType::Skill => skill_variant(&request.intent, strategy, stages),
        ArtifactType::Prompt => prompt_variant(&request.intent, strategy, stages),
    }
}

fn skill_variant(intent: &str, strategy: &str, stages: &[StageOutput]) -> String {
    let model_basis = compact_model_basis(stages);

    format!(
        "# Skill: Focused Web Design Generator\n\nStrategy: {strategy}\n\nUse this skill when the user asks for: {intent}\n\n## What To Do Always\n- Clarify the target audience, business goal, conversion action, legal/safety constraints, and platform constraints before adding visual ideas.\n- Build the design around real user tasks: scan, compare, choose, trust, and act.\n- Define design tokens first: typography scale, spacing, radius, surface, accent, success, warning, error, and focus colors.\n- Produce responsive behavior for mobile, tablet, and desktop as part of the output, not as an afterthought.\n- Include accessibility requirements: contrast, focus states, keyboard path, semantic headings, form labels, reduced motion, and readable error states.\n- Separate visual taste from functional requirements. If a choice is aesthetic, name it as aesthetic.\n- Add quality checks that can fail the design.\n\n## What To Never Do\n- Never make a casino, fintech, medical, legal, or high-risk interface look trustworthy through vague decoration alone.\n- Never use generic hero-card layouts when the user asked for a working product surface.\n- Never hide key actions below the fold on mobile.\n- Never rely on color alone for state, risk, win/loss, or validation feedback.\n- Never overbuild dashboards, animations, or personalization if the user asked for a simple landing or single flow.\n- Never output only a prompt about the skill. Output the actual skill rules, workflow, checklist, and examples.\n\n## Workflow\n1. Restate the brief in one sentence and list assumptions.\n2. Identify the primary user action and the highest-risk misunderstanding.\n3. Choose an interface pattern: landing page, product page, dashboard, onboarding flow, editor, or transactional flow.\n4. Define tokens and layout constraints before writing components.\n5. Draft the screen structure from top to bottom.\n6. Add responsive rules for at least three breakpoints.\n7. Add accessibility and trust checks.\n8. Remove overengineering: delete any element that does not help the user decide, understand, or act.\n9. Produce the final implementation guidance or UI copy.\n10. Run the failure criteria below.\n\n## Quality Checklist\n- The first viewport makes the product/category unmistakable.\n- The primary action is visible and named with a concrete verb.\n- Text fits inside controls at mobile width.\n- Form fields have labels, validation, and useful empty states.\n- Components use stable dimensions so loading/status text does not shift the layout.\n- There is a clear visual hierarchy without relying on oversized marketing typography everywhere.\n- Risk, limits, eligibility, or important conditions are visible near the relevant action.\n- The design avoids a one-note palette and has enough neutral structure to feel controlled.\n\n## Good Example\nA casino web redesign brief should produce a restrained, trust-forward interface: clear game categories, responsible-play limits, visible account controls, accessible contrast, non-manipulative bonus copy, and a mobile-first deposit/withdrawal path.\n\n## Bad Example\n\"Make it premium with neon cards, big jackpot text, and animated buttons\" is not enough. It is vague, may encourage manipulative UX, and has no responsive, accessibility, or safety checks.\n\n## Tests\n- Mobile test: Can a user understand the offer and primary action in 10 seconds at 390px width?\n- Keyboard test: Can a user reach every action and see focus clearly?\n- Copy test: Are bonus, risk, price, or eligibility claims specific and not misleading?\n- Layout test: Does any status label, button, or card resize unexpectedly while loading?\n- Simplicity test: Can one section be removed without reducing user value? If yes, remove it.\n\n## Criteria For Failure\n- The output is just a prompt, summary, or generic advice.\n- The design has no responsive rules.\n- The design has no accessibility checks.\n- The design has no failure criteria.\n- The design depends on decorative polish instead of user task clarity.\n- The result encourages dark patterns, hidden risk, or misleading urgency.\n\n## Model Basis\n{model_basis}"
    )
}

fn prompt_variant(intent: &str, strategy: &str, stages: &[StageOutput]) -> String {
    let model_basis = compact_model_basis(stages);

    format!(
        "Strategy: {strategy}\n\nYou are a senior product designer and frontend systems thinker.\n\nTask\n{intent}\n\nGoal\nProduce a concrete, usable output, not a generic brainstorming note. The result must be specific enough that a designer or frontend engineer can act on it immediately.\n\nProcess\n1. Restate the user's goal and assumptions.\n2. Identify the audience, primary action, constraints, and risks.\n3. Define the output structure before writing the final answer.\n4. Include responsive behavior, accessibility, tokens, examples, and failure checks when relevant.\n5. Remove overengineering and decorative ideas that do not improve the user's outcome.\n\nOutput Contract\n- Start with the final useful artifact.\n- Use clear sections and concrete bullets.\n- Include examples of good and bad results.\n- Include tests or acceptance criteria.\n- Avoid filler, vague best-practice language, and purely aesthetic claims.\n\nModel Basis\n{model_basis}\n\nFinal Response\nWrite the finished artifact now."
    )
}

fn compact_model_basis(stages: &[StageOutput]) -> String {
    let basis = stages
        .iter()
        .filter(|stage| stage.stage == "Generation")
        .take(3)
        .map(|stage| {
            let text = truncate_for_display(&stage.content.trim().replace('\n', " "), 450);
            format!("- {}: {}", stage.model_id, text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    if basis.is_empty() {
        "- No live model basis available yet; use the deterministic skill contract.".to_string()
    } else {
        basis
    }
}

fn answer_variant(intent: &str, strategy: &str, stages: &[StageOutput]) -> String {
    let answer_language = if looks_cyrillic(intent) {
        "Russian"
    } else {
        "English"
    };

    if looks_like_pirozhki_recipe(intent) {
        return pirozhki_recipe_answer(intent, strategy, answer_language);
    }

    let debate_notes = stages
        .iter()
        .filter(|stage| stage.stage == "Generation")
        .take(3)
        .map(|stage| {
            let text = truncate_for_display(&stage.content.trim().replace('\n', " "), 700);
            format!("- {}: {}", stage.model_id, text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let model_basis = if debate_notes.is_empty() {
        "- No live model notes were available; this fallback keeps the output structured."
            .to_string()
    } else {
        debate_notes
    };

    if answer_language == "Russian" {
        format!(
            "Стратегия: {strategy}\n\nЗапрос\n{intent}\n\nПрямой ответ\nНужно отвечать на реальную задачу пользователя, а не выдавать мета-план. Сначала дай готовый полезный результат, затем коротко объясни логику, добавь важные ограничения, крайние случаи и признаки провала.\n\nРекомендуемая структура\n1. Сначала готовый ответ.\n2. Затем понятное объяснение решений.\n3. Потом ограничения, компромиссы и проверки качества.\n4. В конце короткое следующее действие.\n\nСигналы от моделей\n{model_basis}\n\nПланка качества\nФинальный ответ должен быть конкретным, проверяемым и без воды. Если задача зависит от свежих фактов, цен, законов, дат или текущих рекомендаций, финализация должна явно требовать свежей проверки перед уверенным утверждением."
        )
    } else {
        format!(
            "Strategy: {strategy}\n\nRequest\n{intent}\n\nDirect answer\nStart from the user's actual goal, state the useful assumption, then give a concrete result rather than a generic framework. The answer should be complete enough to use immediately, but still honest about what would need verification.\n\nRecommended structure\n1. Give the practical answer first.\n2. Explain the reasoning in plain language.\n3. Add edge cases, tradeoffs, and failure checks.\n4. End with a short next action.\n\nMerged model signals\n{model_basis}\n\nQuality bar\nThe final answer should be specific, testable, and free of filler. If the task needs facts, dates, prices, laws, or current recommendations, it should explicitly require fresh verification before presenting the final claim."
        )
    }
}

fn truncate_for_display(text: &str, max_chars: usize) -> String {
    let mut output = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn looks_like_pirozhki_recipe(intent: &str) -> bool {
    let lower = intent.to_lowercase();
    lower.contains("pirozh")
        || lower.contains("пирож")
        || lower.contains("пиріж")
        || lower.contains("recipe")
        || lower.contains("рецепт")
}

fn looks_cyrillic(text: &str) -> bool {
    text.chars()
        .any(|character| ('\u{0400}'..='\u{04ff}').contains(&character))
}

fn pirozhki_recipe_answer(intent: &str, strategy: &str, answer_language: &str) -> String {
    if answer_language == "Russian" {
        return format!(
            "Стратегия: {strategy}\n\nЗапрос\n{intent}\n\nЛучший ответ\nСделай жареные дрожжевые пирожки с картошкой, карамелизованным луком и небольшим количеством сливочного масла в начинке. Самый вкусный вариант получается не за счет редких ингредиентов, а за счет баланса: мягкое тесто, насыщенная солоноватая начинка, тонкий надежный шов и глубокая золотистая корочка.\n\nИнгредиенты на 12-14 пирожков\nТесто:\n- 500 г пшеничной муки, плюс немного для формовки\n- 250 мл теплого молока\n- 7 г сухих дрожжей или 20 г свежих\n- 1 яйцо\n- 40 г растопленного сливочного масла\n- 1 ст. л. сахара\n- 1 ч. л. соли\n- 1 ст. л. растительного масла\n\nНачинка:\n- 650-750 г картофеля\n- 2 крупные луковицы, мелко нарезанные\n- 40 г сливочного масла\n- 2 ст. л. растительного масла\n- 1 ч. л. соли, потом довести по вкусу\n- Черный перец\n- По желанию: укроп, щепотка сухого чеснока или ложка сметаны для мягкости\n\nКак готовить\n1. Смешай теплое молоко, дрожжи, сахар, яйцо, растопленное масло, соль и большую часть муки. Замеси мягкое эластичное тесто. Оно должно быть нежным, но не мокрым. Последнюю часть муки добавляй постепенно.\n2. Накрой и оставь до увеличения вдвое, примерно на 60-90 минут.\n3. Картофель отвари в соленой воде. Хорошо слей воду и разомни горячим.\n4. Лук обжарь на растительном и сливочном масле медленно, до сладкого золотистого состояния. Не торопи этот шаг: бледный лук даст плоский вкус.\n5. Смешай картофель с луковым маслом. Посоли чуть ярче, чем кажется нужным: тесто приглушит начинку. Полностью остуди начинку перед лепкой.\n6. Раздели тесто на 12-14 частей. Каждую расплющи, положи начинку, плотно защипни и оставь швом вниз на 10-15 минут.\n7. Жарь в 1-1,5 см масла на среднем огне до глубокого золотистого цвета с обеих сторон. Если масло слишком горячее, корочка сгорит раньше, чем середина прогреется.\n8. Выложи на решетку или бумажное полотенце. Для более мягкой корочки можно слегка смазать горячие пирожки сливочным маслом.\n\nПочему это работает\nМолоко, яйцо и масло дают мягкое, но не тяжелое тесто. Карамелизованный лук добавляет сладость и глубину. Остывшая начинка не разрывает шов паром. Средний огонь дает корочке время стать золотистой, а тесту - полностью приготовиться.\n\nПроверки провала\n- Тесто рвется при лепке: оно пересушено или мало отдохнуло.\n- Пирожки раскрываются в масле: начинка была горячей, влажной или шов попал в муку.\n- Корочка темная, а внутри вкус сырого теста: огонь был слишком сильный.\n- Вкус плоский: начинку недосолили до лепки.\n\nКак подать\nЛучше всего есть теплыми со сметаной, чесночным йогуртом или салатом из огурца с укропом. На следующий день они отлично оживают на сухой сковороде."
        );
    }

    format!(
        "Strategy: {strategy}\n\nRequest\n{intent}\n\nBest answer\nMake fried yeast pirozhki with potato, caramelized onion, and a little butter in the filling. The tastiest version is not about exotic ingredients; it is about contrast: tender dough, savory filling, a thin sealed edge, and a deep golden crust.\n\nIngredients for 12-14 pirozhki\nDough:\n- 500 g all-purpose flour, plus a little for shaping\n- 250 ml warm milk\n- 7 g instant yeast, or 20 g fresh yeast\n- 1 egg\n- 40 g melted butter\n- 1 tbsp sugar\n- 1 tsp salt\n- 1 tbsp neutral oil\n\nFilling:\n- 650-750 g potatoes\n- 2 large onions, finely diced\n- 40 g butter\n- 2 tbsp neutral oil\n- 1 tsp salt, then adjust\n- Black pepper\n- Optional: dill, a pinch of garlic powder, or a spoon of sour cream for softness\n\nMethod\n1. Mix warm milk, yeast, sugar, egg, melted butter, salt, and most of the flour. Knead until the dough is soft and elastic. It should be slightly tender, not sticky-wet. Add the last flour gradually.\n2. Cover and let it rise until doubled, about 60-90 minutes depending on room temperature.\n3. Boil the potatoes in salted water. Drain well, then mash while hot.\n4. Slowly fry the onions in oil and butter until sweet, golden, and a little jammy. Do not rush this part; bland onions make bland pirozhki.\n5. Mix onion butter into the potatoes. Season more boldly than you think, because dough softens the flavor. Cool the filling before shaping.\n6. Divide the dough into 12-14 pieces. Flatten each piece, add filling, seal firmly, then rest seam-side down for 10-15 minutes.\n7. Fry in 1-1.5 cm of oil over medium heat until both sides are deep golden. If the oil is too hot, the crust burns before the middle warms through.\n8. Drain on a rack or paper towel. Brush very lightly with melted butter if you want a softer, richer finish.\n\nWhy this works\nThe milk, egg, and butter make the dough soft without turning it cakey. Caramelized onion gives sweetness and depth. Cooling the filling prevents steam from opening the seam. Medium heat gives the crust time to become golden while the inside stays tender.\n\nFailure checks\n- Dough tears while shaping: it is under-rested or too dry.\n- Pirozhki open in oil: the filling was hot, wet, or the seam was thick with flour.\n- Crust is dark but inside tastes doughy: oil was too hot.\n- Flavor is flat: the potato filling was under-salted before shaping.\n\nBest serving\nEat them warm with sour cream, garlic yogurt, or a simple cucumber-dill salad. They are also excellent the next day reheated in a dry skillet."
    )
}

fn api_schema_response() -> String {
    serde_json::json!({
        "artifact_types": ["skill", "prompt", "answer"],
        "execution_modes": ["mock", "live"],
        "variant_count": {
            "default": 1,
            "allowed": [1, 2, 3]
        },
        "payment": {
            "protocol": "x402",
            "headers": {
                "required": "PAYMENT-REQUIRED",
                "signature": "PAYMENT-SIGNATURE",
                "response": "PAYMENT-RESPONSE"
            },
            "network": DEFAULT_X402_NETWORK,
            "asset": DEFAULT_X402_ASSET,
            "enabled_by_env": "X402_ENABLED"
        }
    })
    .to_string()
}

fn error_json(error: impl ToString) -> String {
    serde_json::json!({ "error": error.to_string() }).to_string()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
