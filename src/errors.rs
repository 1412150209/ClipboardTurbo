use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to Initialize")]
    InitializeFailed,
    #[error("Something went wrong when running")]
    RunningError
}