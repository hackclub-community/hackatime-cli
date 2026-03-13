use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, oneshot};
use url::Url;

use crate::config::AppConfig;

const AUTHORIZE_URL: &str = "https://hackatime.hackclub.com/oauth/authorize";
const TOKEN_URL: &str = "https://hackatime.hackclub.com/oauth/token";
const OAUTH_SUCCESS_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Hackatime Terminal Viewer</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #141617;
      --panel: #1d2021;
      --panel-soft: #282828;
      --line: #3c3836;
      --text: #fbf1c7;
      --muted: #bdae93;
      --primary: #d8a657;
      --primary-strong: #e78a4e;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      min-height: 100vh;
      display: grid;
      place-items: center;
      background: linear-gradient(180deg, #111314 0%, var(--bg) 100%);
      color: var(--text);
      font-family:
        Inter,
        ui-sans-serif,
        system-ui,
        -apple-system,
        BlinkMacSystemFont,
        "Segoe UI",
        sans-serif;
    }

    main {
      width: min(92vw, 46rem);
      padding: 2.5rem;
    }

    .card {
      border: 1px solid var(--line);
      background: linear-gradient(180deg, var(--panel) 0%, #181b1c 100%);
      border-radius: 18px;
      overflow: hidden;
      box-shadow:
        0 14px 48px rgba(0, 0, 0, 0.35),
        0 0 0 1px rgba(255, 255, 255, 0.02) inset;
    }

    .topbar {
      display: flex;
      align-items: center;
      gap: 0.6rem;
      padding: 1rem 1.25rem;
      background: rgba(255, 255, 255, 0.02);
      border-bottom: 1px solid var(--line);
    }

    .dot {
      width: 0.72rem;
      height: 0.72rem;
      border-radius: 999px;
    }

    .dot-primary {
      background: var(--primary);
    }

    .dot-warm {
      background: var(--primary-strong);
    }

    .dot-soft {
      background: #89b482;
    }

    .content {
      padding: 2.25rem 2rem;
    }

    .eyebrow {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      margin: 0 0 1rem;
      padding: 0.42rem 0.72rem;
      border: 1px solid rgba(216, 166, 87, 0.28);
      border-radius: 999px;
      background: rgba(216, 166, 87, 0.08);
      color: var(--primary);
      font-size: 0.78rem;
      font-weight: 700;
      letter-spacing: 0.14em;
      text-transform: uppercase;
    }

    h1 {
      margin: 0 0 0.75rem;
      font-size: clamp(2.1rem, 4vw, 3.6rem);
      line-height: 1.02;
      font-weight: 800;
      letter-spacing: -0.04em;
    }

    p {
      margin: 0;
      max-width: 36rem;
      color: var(--muted);
      font-size: 1.02rem;
      line-height: 1.7;
    }

    .rule {
      width: 100%;
      height: 1px;
      margin: 1.5rem 0;
      background: linear-gradient(90deg, var(--primary), rgba(216, 166, 87, 0.12) 72%, transparent 100%);
    }

    strong {
      color: var(--text);
      font-weight: 700;
    }
  </style>
</head>
<body>
  <main>
    <section class="card">
      <div class="topbar">
        <span class="dot dot-primary"></span>
        <span class="dot dot-warm"></span>
        <span class="dot dot-soft"></span>
      </div>
      <div class="content">
        <p class="eyebrow">OAuth Complete</p>
        <h1>Hackatime Terminal Viewer</h1>
        <div class="rule"></div>
        <p><strong>You're signed in.</strong> You can close this tab and return to the terminal to see your stats.</p>
      </div>
    </section>
  </main>
</body>
</html>
"#;

#[derive(Debug, Clone)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

#[derive(Debug, Clone)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

#[derive(Clone)]
struct CallbackState {
    sender: Arc<Mutex<Option<oneshot::Sender<Result<OAuthCallback, String>>>>>,
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub fn generate_pkce_pair() -> PkcePair {
    let mut random = [0_u8; 32];
    OsRng.fill_bytes(&mut random);

    let verifier = URL_SAFE_NO_PAD.encode(random);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    PkcePair {
        verifier,
        challenge,
    }
}

pub fn random_state() -> String {
    let mut random = [0_u8; 24];
    OsRng.fill_bytes(&mut random);
    URL_SAFE_NO_PAD.encode(random)
}

pub async fn authorize(config: &AppConfig) -> Result<String> {
    let pkce = generate_pkce_pair();
    let state = random_state();
    let callback = listen_for_callback(&config.redirect_uri);
    let auth_url = build_authorize_url(config, &pkce, &state)?;

    open::that(auth_url.as_str()).context("failed to open browser for OAuth login")?;

    let callback_data = callback.await?.map_err(anyhow::Error::msg)?;
    if callback_data.state != state {
        anyhow::bail!("OAuth state mismatch; refusing to continue");
    }

    exchange_code(config, &callback_data.code, &pkce.verifier).await
}

fn build_authorize_url(config: &AppConfig, pkce: &PkcePair, state: &str) -> Result<Url> {
    let mut url = Url::parse(AUTHORIZE_URL)?;
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &config.scopes)
        .append_pair("state", state)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url)
}

fn listen_for_callback(
    redirect_uri: &str,
) -> impl std::future::Future<Output = Result<Result<OAuthCallback, String>>> {
    let redirect = redirect_uri.to_string();

    async move {
        let parsed = Url::parse(&redirect)?;
        let host = parsed
            .host_str()
            .context("redirect URI is missing a host")?
            .to_string();
        let port = parsed
            .port_or_known_default()
            .context("redirect URI is missing a port")?;
        let path = parsed.path().to_string();
        let addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .with_context(|| format!("invalid redirect socket address: {host}:{port}"))?;

        let (sender, receiver) = oneshot::channel::<Result<OAuthCallback, String>>();
        let state = CallbackState {
            sender: Arc::new(Mutex::new(Some(sender))),
        };

        let app = Router::new()
            .route(&path, get(handle_callback))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .with_context(|| format!("failed to bind OAuth callback listener on {addr}"))?;

        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .map_err(anyhow::Error::from)
        });

        let result = receiver.await.context("did not receive OAuth callback")?;
        server.abort();
        Ok(result)
    }
}

async fn handle_callback(
    State(state): State<CallbackState>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<&'static str> {
    let callback_params = CallbackParams {
        code: params.get("code").cloned(),
        state: params.get("state").cloned(),
        error: params.get("error").cloned(),
        error_description: params.get("error_description").cloned(),
    };

    let result = if let Some(error) = callback_params.error {
        Err(callback_params
            .error_description
            .unwrap_or_else(|| format!("OAuth error: {error}")))
    } else {
        match (callback_params.code, callback_params.state) {
            (Some(code), Some(state)) => Ok(OAuthCallback { code, state }),
            _ => Err("Missing code or state in OAuth callback".to_string()),
        }
    };

    if let Some(sender) = state.sender.lock().await.take() {
        let _ = sender.send(result);
    }

    Html(OAUTH_SUCCESS_HTML)
}

async fn exchange_code(config: &AppConfig, code: &str, verifier: &str) -> Result<String> {
    let client = Client::new();
    let response = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", config.client_id.as_str()),
            ("redirect_uri", config.redirect_uri.as_str()),
            ("code", code),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .context("failed to exchange OAuth code for token")?
        .error_for_status()
        .context("Hackatime rejected the OAuth token exchange")?;

    let payload = response
        .json::<TokenResponse>()
        .await
        .context("failed to decode OAuth token response")?;

    Ok(payload.access_token)
}
