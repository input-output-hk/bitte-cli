use thiserror::Error;

#[derive(Error, Debug)]
pub enum BitteError {
    #[error("timeout elapsed")]
    Timeout(#[from] tokio::time::error::Elapsed),
    #[error("exhausted {0} attempts")]
    ExhaustedAttempts(usize),
    #[error("connection with {0} failed")]
    ConnectionFailed(#[from] std::io::Error),
    #[error("unknown error")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::BitteError;

    #[tokio::test]
    // This silly test is to make sure we can match
    // specific errors!
    async fn test_unknown() {
        let result: Result<(), BitteError> = Err(BitteError::Unknown);
        assert!(result.is_err());
    }
}