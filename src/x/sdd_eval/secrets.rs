#[derive(Debug, Default, Clone)]
pub struct SecretSet {
    secrets: Vec<String>,
}

impl SecretSet {
    pub fn new() -> Self {
        Self {
            secrets: Vec::new(),
        }
    }

    pub fn push(&mut self, secret: String) {
        if secret.trim().is_empty() {
            return;
        }
        if !self.secrets.contains(&secret) {
            self.secrets.push(secret);
        }
    }

    pub fn redact(&self, input: &str) -> String {
        let mut out = input.to_string();
        for s in &self.secrets {
            if s.is_empty() {
                continue;
            }
            out = out.replace(s, "[REDACTED]");
        }
        out
    }
}
