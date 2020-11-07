#[derive(Debug, thiserror::Error)]
pub enum Ros2wsError {
    #[error("the type of the data for key `{0}` should be {1}")]
    InvalidManifestFile(String, String),
}
