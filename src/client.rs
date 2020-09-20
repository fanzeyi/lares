use thiserror::Error;

use crate::error::Result;

const MAXIMUM_REDIRECTION: u8 = 5;
const USER_AGENT: &'static str = "LaresBot/0.2 (+https://github.com/fanzeyi/lares)";

pub struct HttpClient;

#[derive(Debug, Error)]
pub enum HttpClientError {
    #[error("Too many redirections")]
    TooManyRedirections,

    #[error("Missing location header for redirection response")]
    MissingLocationHeader,

    #[error("Unexpected status code: {}", _0)]
    UnexpectedStatusCode(http::status::StatusCode),

    #[error("Surf error")]
    SurfError(#[from] surf::Exception),
}

impl HttpClient {
    pub async fn get(url: &str) -> Result<Vec<u8>> {
        let mut url = url.to_owned();
        let mut redirection_count = 0;

        loop {
            let mut response = match surf::get(&url)
                .set_header("User-Agent", USER_AGENT)
                .set_header("Content-Length", "0")
                .await
            {
                Ok(resp) => resp,
                Err(e) => return Err(HttpClientError::from(e).into()),
            };
            let status = response.status();

            if status.is_success() {
                break Ok(response.body_bytes().await?);
            }

            if status.is_redirection() {
                if redirection_count > MAXIMUM_REDIRECTION {
                    break Err(HttpClientError::TooManyRedirections.into());
                }

                redirection_count += 1;

                if let Some(location) = response.header("Location") {
                    url = location.to_owned();
                } else {
                    break Err(HttpClientError::MissingLocationHeader.into());
                }
            } else {
                break Err(HttpClientError::UnexpectedStatusCode(status).into());
            }
        }
    }
}
