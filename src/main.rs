use anyhow::{Context, Result, anyhow, bail};
use lambda_http::{service_fn, Error, lambda_runtime::{self, LambdaEvent}, tracing::{self, Level}};
use reqwest;
use serde::Serialize;
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

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

struct Timer(Option<std::time::Instant>);
impl Timer {
    fn start() -> Timer {
        Timer(if tracing::enabled!(Level::DEBUG) { Some(std::time::Instant::now()) } else { None })
    }
    fn end(self, msg: &str) {
        if let Some(start) = self.0 {
            tracing::debug!("{} in {:.2?}", msg, start.elapsed());
        }
    }
}

async fn lookup_url() -> Result<Url> {
    let timer = Timer::start();
    let base_url = env::var("BASE_URL")
        .context("Please set a BASE_URL environment variable")?;
    let mut base_url = Url::parse(base_url.trim_end_matches('/'))?;
    let host = base_url.host_str().ok_or(anyhow!("cannot parse host part of BASE_URL {}", base_url))?;
    let _ = tokio::net::lookup_host(format!("{}:443", host)).await?;

    base_url.set_path("/api/alexa/smart_home");

    timer.end("lookup_url() resolved IP");
    Ok(base_url)
}

async fn build_reqwest_client(event: &LambdaEvent<Value>) -> Result<(reqwest::Client, String)> {
    let timer = Timer::start();
    if tracing::enabled!(Level::TRACE) {
        let evt = serde_json::to_string_pretty(&event.payload)?;
        tracing::trace!("Event: {}", evt);
    }

    let directive = &event.payload["directive"];
    if directive.is_null() {
        bail!("Malformed request - missing directive");
    }
    let payload_version = directive["header"]["payloadVersion"].as_str().unwrap_or_default();
    if payload_version != "3" {
        bail!("Only payloadVersion == \"3\" is supported, got {}", payload_version);
    }

    let mut scope = &directive["endpoint"]["scope"];
    if scope.is_null() {
        scope = &directive["payload"]["grantee"];
    }
    if scope.is_null() {
        scope = &directive["payload"]["scope"];
    }
    if scope.is_null() {
        bail!("Malformed request - missing one between endpoint.scope, payload.grantee, or payload.scope");
    }
    if scope["type"].as_str().unwrap_or_default() != "BearerToken" {
        bail!("Malformed request - endpoint.scope.type only supports BearerToken");
    }

    let token = &scope["token"].as_str();
    let token = if token.is_none() && tracing::enabled!(Level::DEBUG) {
        env::var("LONG_LIVED_ACCESS_TOKEN").context("No token found in event, please provide a LONG_LIVEDF_ACCESS_TOKEN instead")?
    } else {
        token.ok_or(anyhow!("Malformed request - missing auth token"))?.into()
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
        .build()?;
    timer.end("build_request_client() completed");
    Ok((client, token))
}

async fn handler(event: LambdaEvent<Value>) -> Result<Value> {
    let (base_url, (client, token)) = tokio::try_join!(
        lookup_url(),
        build_reqwest_client(&event))?;

    let response = client.post(base_url.as_str())
        .header("Authorization", format!("Bearer {}", token))
        .json(&event.payload)
        .send()
        .await?;
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
                    message: response.text().await?,
                }
            }
        };
        return Ok(serde_json::to_value(&val)?);
    }

    Ok(response.json::<Value>().await?)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    lambda_runtime::run(service_fn(handler)).await
}
