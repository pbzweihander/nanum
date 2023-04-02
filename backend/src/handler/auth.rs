use anyhow::Result;
use axum::{
    async_trait,
    extract::{FromRequestParts, Query, State},
    headers,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing, RequestPartsExt, Router, TypedHeader,
};
use headers::HeaderMap;
use jsonwebtoken::{decode, encode, Validation};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::config::CONFIG;

use super::AppState;

static SESSION_COOKIE_NAME: &str = "session";

pub fn create_oauth_client() -> BasicClient {
    BasicClient::new(
        ClientId::new(CONFIG.github_client_id.clone()),
        Some(ClientSecret::new(CONFIG.github_client_secret.clone())),
        AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap()),
    )
}

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/github", routing::get(handle_get_github))
        .route("/authorized", routing::get(handle_get_authorized))
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub primary_email: String,
    pub emails: Vec<String>,
    pub exp: i64,
}

pub enum UserRejection {
    NotAuthorized,
    Error(&'static str),
    Forbidden,
}

impl IntoResponse for UserRejection {
    fn into_response(self) -> Response {
        match self {
            Self::NotAuthorized => Redirect::to("/auth/github").into_response(),
            Self::Error(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
            Self::Forbidden => (StatusCode::FORBIDDEN, "user not allowed").into_response(),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = UserRejection;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let cookies: Option<TypedHeader<headers::Cookie>> =
            parts.extract().await.map_err(|error| {
                tracing::error!(%error, "failed to extract Cookie header");
                UserRejection::Error("failed to extract Cookie header")
            })?;
        let session_cookie = cookies
            .as_ref()
            .and_then(|cookies| cookies.get(SESSION_COOKIE_NAME))
            .ok_or(UserRejection::NotAuthorized)?;

        let mut jwt_validation = Validation::default();
        jwt_validation.validate_exp = true;
        let user_data = decode::<User>(session_cookie, &CONFIG.jwt_secret.1, &jwt_validation)
            .map_err(|error| {
                tracing::error!(%error, "failed to decode JWT session token");
                UserRejection::NotAuthorized
            })?;
        let user = user_data.claims;

        for allowed_email in &CONFIG.allowed_emails {
            if user.emails.contains(allowed_email) {
                return Ok(user);
            }
        }

        Err(UserRejection::Forbidden)
    }
}

#[derive(Deserialize)]
struct GetGitHubReq {
    #[serde(default)]
    redirect: Option<String>,
}

async fn handle_get_github(
    State(state): State<AppState>,
    Query(req): Query<GetGitHubReq>,
) -> Redirect {
    let mut redirect_url = CONFIG.public_url.join("./auth/authorized").unwrap();
    if let Some(redirect) = req.redirect {
        redirect_url.set_query(Some(&format!("redirect={}", redirect)));
    }
    let (auth_url, _csrf_token) = state
        .oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_string()))
        .set_redirect_uri(std::borrow::Cow::Owned(RedirectUrl::from_url(redirect_url)))
        .url();
    Redirect::to(auth_url.as_ref())
}

#[derive(Deserialize)]
struct AuthRequest {
    code: String,
}

#[derive(Deserialize, Debug)]
struct GitHubEmailsResp {
    email: String,
    verified: bool,
    primary: bool,
}

async fn handle_get_authorized(
    Query(req): Query<AuthRequest>,
    State(state): State<AppState>,
) -> Result<(HeaderMap, Redirect), (StatusCode, &'static str)> {
    let token = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(req.code))
        .request_async(async_http_client)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to request OAuth");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to request OAuth")
        })?;

    let resp: Vec<GitHubEmailsResp> = state
        .http_client
        .get("https://api.github.com/user/emails")
        .bearer_auth(token.access_token().secret())
        .header(header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to request GitHub");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to request GitHub",
            )
        })?
        .json()
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to decode GitHub response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to decode GitHub response",
            )
        })?;

    let mut primary_email = None;
    let mut emails = Vec::with_capacity(resp.len());
    for email in resp {
        if email.primary && primary_email.is_none() {
            primary_email = Some(email.email.clone());
        }
        if email.verified {
            emails.push(email.email);
        }
    }
    if emails.is_empty() {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "email is empty"));
    }
    let primary_email = primary_email.unwrap_or_else(|| emails[0].clone());

    let now = OffsetDateTime::now_utc();
    let exp = (now + Duration::days(1)).unix_timestamp();

    let user = User {
        primary_email,
        emails,
        exp,
    };

    let session_token =
        encode(&Default::default(), &user, &CONFIG.jwt_secret.0).map_err(|error| {
            tracing::error!(%error, "failed to encode JWT session token");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to encode JWT session token",
            )
        })?;

    let cookie = format!(
        "{}={}; SameSite=Lax; Path=/",
        SESSION_COOKIE_NAME, session_token
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    Ok((headers, Redirect::to("/")))
}
