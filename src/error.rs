use thiserror::Error;

#[derive(Debug, Error)]
pub enum SwapxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Prompt error: {0}")]
    Dialoguer(#[from] dialoguer::Error),

    #[error("Config error: {0}")]
    Config(String),
}
