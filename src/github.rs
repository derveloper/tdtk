//!
//! This example showcases the Github OAuth2 process for requesting access to the user's public repos and
//! email address.
//!
//! Before running it, you'll need to generate your own Github OAuth2 credentials.
//!
//! In order to run the example call:
//!
//! ```sh
//! GITHUB_CLIENT_ID=xxx GITHUB_CLIENT_SECRET=yyy cargo run --example github
//! ```
//!
//! ...and follow the instructions.
//!

use anyhow::{Context, Result};
use oauth2::{AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};
use oauth2::basic::{BasicClient, BasicTokenResponse};
// Alternatively, this can be `oauth2::curl::http_client` or a custom client.
use oauth2::reqwest::async_http_client;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use url::Url;

pub(crate) async fn get_github_token() -> Result<BasicTokenResponse> {
    let github_client_id = ClientId::new(
        env!("GH_CLIENT_ID").to_string()
    );
    let github_client_secret = ClientSecret::new(
        env!("GH_CLIENT_SECRET").to_string()
    );
    let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
        .with_context(|| "Invalid authorization endpoint URL")?;
    let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
        .with_context(|| "Invalid token endpoint URL")?;

    // Set up the config for the Github OAuth2 process.
    let client = BasicClient::new(
        github_client_id,
        Some(github_client_secret),
        auth_url,
        Some(token_url),
    )
        // This example will be running its own server at localhost:8080.
        // See below for the server implementation.
        .set_redirect_uri(
            RedirectUrl::new("http://localhost:8080".to_string()).with_context(|| "Invalid redirect URL")?,
        );

    // Generate the authorization URL to which we'll redirect the user.
    let (authorize_url, _) = client
        .authorize_url(CsrfToken::new_random)
        // This example is requesting access to the user's public repos and email.
        .add_scope(Scope::new("repo".to_string()))
        .add_scope(Scope::new("delete_repo".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    open::that(authorize_url.to_string()).unwrap();

    // A very naive implementation of the redirect server.
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let token_res;
    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let code;
            {
                let mut reader = BufReader::new(&mut stream);

                let mut request_line = String::new();
                reader.read_line(&mut request_line).await.unwrap();

                let redirect_url = request_line.split_whitespace().nth(1).unwrap();
                let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "code"
                    })
                    .unwrap();

                let (_, value) = code_pair;
                code = AuthorizationCode::new(value.into_owned());
            }

            let message = "<script type=\"text/javascript\">\
            setTimeout(\"window.close();\", 150);</script>\
            Go back to your terminal :)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).await.unwrap();

            // Exchange the code with a token.
            token_res = client
                .exchange_code(code)
                .request_async(async_http_client)
                .await;

            // The server will terminate itself after collecting the first code.
            break;
        }
    }

    token_res.with_context(|| "Error getting token")
}