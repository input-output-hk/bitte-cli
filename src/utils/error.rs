#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    #[error("timeout elapsed")]
    Timeout(#[from] tokio::time::error::Elapsed),
    #[error("exhausted {0} attempts")]
    ExhaustedAttempts(usize),
    #[error("connection with {0} failed")]
    ConnectionFailed(#[from] std::io::Error),
    #[error("environment variable")]
    EnvVar(#[from] std::env::VarError),
    #[error("github token missin in ~/.netrc file")]
    NoGithubToken,
    #[error("error parsing json")]
    Serde(#[from] serde_json::Error),
    #[error("error making rest api request")]
    Restson(#[from] restson::Error),
    #[error("couldn't generate terraform config")]
    FailedTerraformConfig,
    #[error("error decoding base64 state")]
    Decode(#[from] base64::DecodeError),
    #[error("error parsing netrc file")]
    Netrc(netrc_rs::Error),
    #[error("couldn't read ~/.netrc")]
    NetrcMissing,
    #[error("error executing external process: {details}")]
    Exe { details: String },
    #[error("current BITTE_PROVIDER is not valid: {provider}")]
    Provider { provider: String },
    #[error("unknown error")]
    Unknown,
}

// NOTE netrc_rs doesn't impl StdError so can't simply `#[from]`
impl From<netrc_rs::Error> for Error {
    fn from(error: netrc_rs::Error) -> Self {
        Error::Netrc(error)
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[tokio::test]
    // This silly test is to make sure we can match
    // specific errors!
    async fn test_unknown() {
        let result: Result<(), Error> = Err(Error::Unknown);
        assert!(result.is_err());
    }
}