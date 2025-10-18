//! SSH integration stubs
//! In a future iteration, real SSH session management will be implemented here.

#[derive(Debug, Clone)]
pub struct SshSessionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SshSessionHandle {
    pub id: uuid::Uuid,
}

impl SshSessionHandle {
    pub fn new() -> Self {
        Self { id: uuid::Uuid::new_v4() }
    }
}
