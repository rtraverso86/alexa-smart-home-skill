use std::env;
use lambda_runtime::{service_fn, tracing::{self, Level}, Error, LambdaEvent};
use serde_json::{json, Value};


async fn handler(event: LambdaEvent<Value>) -> Result<Value, String> {
    if tracing::enabled!(Level::DEBUG) {
        let evt = serde_json::to_string_pretty(&event.payload).or(Err("Could not serialize event.payload"))?;
        tracing::debug!("Event: {}", evt);
    }

    let base_url = env::var("BASE_URL");
    if let Err(e) = base_url {
        tracing::error!("Please set BASE_URL environment variable");
        return Err(format!("{}: please set BASE_URL environment variable", e.to_string()));
    }
    let base_url = base_url.unwrap().trim_end_matches('/');

    let directive = &event.payload["directive"];
    if directive.is_null() {
        return Err("Malformed request - missing directive".into());
    }
    let payload_version = &directive["header"]["payloadVersion"].as_i64();
    if payload_version.unwrap_or(0) != 3 {
        return Err("Only support payloadVersion == 3".into());
    }

    let mut scope = &directive["endpoint"]["scope"];
    if scope.is_null() {
        scope = &directive["payload"]["grantee"];
    }
    if scope.is_null() {
        scope = &directive["payload"]["scope"];
    }
    if (scope.is_null()) {
        return Err("Malformed request - missing endpoint.scope".into());
    }
    if scope["type"].to_string() != "BearerToken" {
        return Err("Malformed request - endpoint.scope.type only supports BearerToken".into());
    }

    let token = &scope["token"];
    if token.is_none() {
        token = env::var("LONG_LIVED_ACCESS_TOKEN").unwr;
    }

    Ok(Value::Null)
    //let first_name = payload["firstName"].as_str().unwrap_or("world");
    //Ok(json!({ "message": format!("Hello, {first_name}!") }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    lambda_runtime::run(service_fn(handler)).await
}

