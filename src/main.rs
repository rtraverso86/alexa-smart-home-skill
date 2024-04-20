use std::{env, time::Duration};
use reqwest;
use lambda_http::{service_fn, Error, lambda_runtime::{self, LambdaEvent}, tracing::{self, Level}};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct ResponseData {
    event: EventData,
}

#[derive(Serialize)]
struct EventData {
    payload: PayloadData,
}

#[derive(Serialize)]
struct PayloadData {
    #[serde(rename = "type")]
    t: String,
    message: String,
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value, String> {
    if tracing::enabled!(Level::TRACE) {
        let evt = serde_json::to_string_pretty(&event.payload).or(Err("Could not serialize event.payload"))?;
        tracing::trace!("Event: {}", evt);
    }

    let base_url = env::var("BASE_URL")
        .map_err(|e| format!("{}: please set the BASE_URL environment variable", e))?;
    let base_url = base_url.trim_end_matches('/');

    let directive = &event.payload["directive"];
    if directive.is_null() {
        return Err("Malformed request - missing directive".into());
    }
    let payload_version = &directive["header"]["payloadVersion"].as_str();
    if payload_version.unwrap_or_default() != "3" {
        return Err("Only support payloadVersion == \"3\"".into());
    }

    let mut scope = &directive["endpoint"]["scope"];
    if scope.is_null() {
        scope = &directive["payload"]["grantee"];
    }
    if scope.is_null() {
        scope = &directive["payload"]["scope"];
    }
    if scope.is_null() {
        return Err("Malformed request - missing endpoint.scope".into());
    }
    if scope["type"].as_str().unwrap_or_default() != "BearerToken" {
        return Err("Malformed request - endpoint.scope.type only supports BearerToken".into());
    }

    let token = &scope["token"].as_str();
    let token = if token.is_none() && tracing::enabled!(Level::DEBUG) {
        env::var("LONG_LIVED_ACCESS_TOKEN").unwrap()
    } else {
        token.ok_or("Malformed request - missing auth token")?.into()
    };

    let disable_ssl_verification = if let Ok(v) = env::var("NOT_VERIFY_SSL") {
        v.parse().unwrap_or(false)
    } else {
        false
    };

    let client = reqwest::ClientBuilder::new()
        .connect_timeout(Duration::from_secs(2))
        .read_timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(disable_ssl_verification)
        .build()
        .map_err(|e| format!("Could not build reqwest client: {}", e))?;
    let response = client.post(format!("{}/api/alexa/smart_home", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&event.payload).or(Err("Could not serialize response body"))?)
        .send()
        .await
        .map_err(|e| format!("An error occurred while awaiting the http response: {}", e))?;
    let response_status = response.status();

    if !response_status.is_success() {
        let val = ResponseData {
            event: EventData {
                payload: PayloadData {
                    t: (if [401, 403].contains(&response_status.as_u16()) {
                            "INVALID_AUTHORIZATION_CREDENTIAL"
                        } else {
                            "INTERNAL_ERROR"
                        }).to_owned(),
                    message: response.text()
                        .await
                        .map_err(|e| format!("Could not extract error response message: {}", e))?,
                }
            }
        };
        return Ok(serde_json::to_value(&val)
            .map_err(|e| format!("Could not deseriialize ResponseData with error info: {}", e))?);
    }

    let response_data = response
        .json::<Value>()
        .await
        .map_err(|e| format!("Could not deserialize response data: {}", e))?;
    Ok(response_data)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    lambda_runtime::run(service_fn(handler)).await
}

