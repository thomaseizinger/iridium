use super::crypto::{make_nonce, Crypto};
use super::{Credentials, Item};
use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use reqwest::blocking::Response;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuthParamsResponse {
    pub pw_cost: u32,
    pub pw_nonce: String,
    pub version: String,
}

#[derive(Deserialize)]
struct User {
    pub uuid: Uuid,
    pub email: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    pub errors: Vec<String>,
}

#[derive(Serialize)]
struct RegistrationRequest {
    pub email: String,
    pub password: String,
    pub pw_cost: u32,
    pub pw_nonce: String,
    pub version: String,
}

#[derive(Serialize)]
struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
struct SignInResponse {
    pub user: User,
    pub token: String,
}

#[derive(Serialize)]
struct SyncRequest {
    pub items: Vec<Item>,
    pub sync_token: Option<String>,
    pub cursor_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SyncResponse {
    pub retrieved_items: Vec<Item>,
    pub saved_items: Vec<Item>,
    pub unsaved: Option<Vec<Item>>,
    pub sync_token: Option<String>,
    pub cursor_token: Option<String>,
}

pub struct Client {
    host: String,
    pub credentials: Credentials,
    pub crypto: Crypto,
    client: reqwest::blocking::Client,
    pub auth_token: String,
    sync_token: Option<String>,
}

fn get_token_from_signin_response(response: Response) -> Result<String> {
    match response.status() {
        StatusCode::OK => {
            let response = response.json::<SignInResponse>()?;
            Ok(response.token)
        }
        _ => {
            let response = response.json::<ErrorResponse>()?;
            Err(anyhow!("{}", response.errors[0]))
        }
    }
}

impl Client {
    /// Create client by registering a new user
    pub fn new_register(host: &str, email: &str, password: &str) -> Result<Client> {
        let credentials = Credentials {
            identifier: email.to_string(),
            cost: 110000,
            nonce: make_nonce(),
            password: password.to_string(),
        };

        let crypto = Crypto::new(&credentials)?;
        let encoded_pw = crypto.password();

        let request = RegistrationRequest {
            email: email.to_string(),
            password: encoded_pw,
            pw_cost: credentials.cost,
            pw_nonce: credentials.nonce.clone(),
            version: "003".to_string(),
        };

        let url = format!("{}/auth", host);
        let client = reqwest::blocking::Client::new();
        let response = client.post(&url).json(&request).send()?;

        Ok(Self {
            host: host.to_string(),
            credentials: credentials,
            crypto: crypto,
            client: client,
            auth_token: get_token_from_signin_response(response)?,
            sync_token: None,
        })
    }

    /// Create client by signing in.
    pub fn new_sign_in(host: &str, email: &str, password: &str) -> Result<Client> {
        let client = reqwest::blocking::Client::new();

        let url = format!("{}/auth/params?email={}", host, email);
        let response = client.get(&url).send()?.json::<AuthParamsResponse>()?;

        let credentials = Credentials {
            identifier: email.to_string(),
            cost: response.pw_cost,
            nonce: response.pw_nonce,
            password: password.to_string(),
        };

        let crypto = Crypto::new(&credentials)?;
        let encoded_pw = crypto.password();

        let request = SignInRequest {
            email: email.to_string(),
            password: encoded_pw,
        };

        let url = format!("{}/auth/sign_in", host);
        let response = client.post(&url).json(&request).send()?;

        Ok(Self {
            host: host.to_string(),
            credentials: credentials,
            crypto: crypto,
            client: client,
            auth_token: get_token_from_signin_response(response)?,
            sync_token: None,
        })
    }

    pub fn sync(&mut self, items: Vec<Item>) -> Result<Vec<Item>> {
        let url = format!("{}/items/sync", &self.host);
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let sync_request = SyncRequest {
            items: items,
            sync_token: self.sync_token.clone(),
            cursor_token: None,
        };

        let response = self.client
            .post(&url)
            .headers(headers)
            .bearer_auth(&self.auth_token)
            .body(serde_json::to_string(&sync_request)?)
            .send()?
            .json::<SyncResponse>()?;

        self.sync_token = response.sync_token;
        Ok(response.retrieved_items)
    }
}
