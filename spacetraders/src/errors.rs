use std::fmt;

#[derive(Debug)]
pub enum GameStatusError {
    ServiceUnavailable,
    HttpError(reqwest::Error),
    ParseError(anyhow::Error),
}

impl fmt::Display for GameStatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GameStatusError::ServiceUnavailable => write!(f, "SpaceTraders API is down for maintenance"),
            GameStatusError::HttpError(ref err) => write!(f, "Http Client error: {}", err),
            GameStatusError::ParseError(ref err) => write!(f, "Error parsing game status response: {}", err),
        }
    }
}

impl From<reqwest::Error> for GameStatusError {
    fn from(err: reqwest::Error) -> Self {
        GameStatusError::HttpError(err)
    }
}

impl From<anyhow::Error> for GameStatusError {
    fn from(err: anyhow::Error) -> Self {
        GameStatusError::ParseError(err)
    }
}
