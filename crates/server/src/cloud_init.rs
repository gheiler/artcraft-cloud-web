//! Fills in artcraftCloud's cloud-init recipe.
//!
//! `templates/artcraft-cloud.yaml` is a copy of `deploy/cloud-init/artcraft-cloud.yaml` from
//! gheiler/artcraft-cloud, so one-click deploys, the pasted recipe and people who install by hand
//! all run the same script. Refresh it with `scripts/sync-cloud-init.sh`. The recipe keeps every
//! setting in an `.env` block; we only rewrite those `KEY=` lines, with values that were validated
//! (domain, email) or generated here (password), so nothing user-typed reaches the YAML as-is.

use types::{valid_domain, valid_email, DeployMode};

pub const TEMPLATE: &str = include_str!("../templates/artcraft-cloud.yaml");

/// What goes into the recipe's `.env`.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// `None` -> the recipe falls back to `<ip>.sslip.io`.
    pub domain: Option<String>,
    pub acme_email: Option<String>,
    /// `None` -> generated on the server at first boot.
    pub admin_password: Option<String>,
    pub mail: bool,
}

impl Settings {
    pub fn new(domain: Option<String>, email: &str, mode: DeployMode) -> Self {
        Settings {
            domain,
            acme_email: Some(email.trim().to_string()).filter(|e| !e.is_empty()),
            admin_password: None,
            mail: mode == DeployMode::DriveMail,
        }
    }
}

/// Returns the recipe with `settings` filled in.
pub fn render(settings: &Settings) -> Result<String, String> {
    let domain = settings.domain.clone().unwrap_or_default();
    if !domain.is_empty() && !valid_domain(&domain) {
        return Err("invalid domain".into());
    }
    let email = settings.acme_email.clone().unwrap_or_default();
    if !email.is_empty() && !valid_email(&email) {
        return Err("invalid email".into());
    }
    let password = settings.admin_password.clone().unwrap_or_default();
    if !password.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("invalid password".into());
    }
    let values = [
        ("ARTCRAFT_DOMAIN", domain),
        ("ARTCRAFT_ACME_EMAIL", email),
        ("ARTCRAFT_ADMIN_PASSWORD", password),
        (
            "ARTCRAFT_MAIL",
            if settings.mail { "1" } else { "0" }.to_string(),
        ),
    ];
    let mut seen = [false; 4];
    let mut out = String::with_capacity(TEMPLATE.len() + 128);
    for line in TEMPLATE.lines() {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        let mut replaced = false;
        for (i, (key, value)) in values.iter().enumerate() {
            if !seen[i] && trimmed.starts_with(&format!("{key}=")) && indent.len() == 6 {
                out.push_str(&format!("{indent}{key}={value}\n"));
                seen[i] = true;
                replaced = true;
                break;
            }
        }
        if !replaced {
            out.push_str(line);
            out.push('\n');
        }
    }
    if seen.iter().any(|s| !s) {
        return Err("the cloud-init template is missing an .env key".into());
    }
    Ok(out)
}

/// A random 24-character alphanumeric password.
pub fn generate_password() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut out = String::with_capacity(24);
    while out.len() < 24 {
        let mut buf = [0u8; 32];
        getrandom::fill(&mut buf).expect("the OS random number generator works");
        for b in buf {
            // Rejection sampling keeps every character equally likely.
            if (b as usize) < 256 - 256 % ALPHABET.len() && out.len() < 24 {
                out.push(ALPHABET[b as usize % ALPHABET.len()] as char);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_line(yaml: &str, key: &str) -> String {
        yaml.lines()
            .find(|l| l.trim_start().starts_with(&format!("{key}=")))
            .unwrap()
            .trim()
            .to_string()
    }

    #[test]
    fn fills_the_env_block() {
        let yaml = render(&Settings {
            domain: Some("cloud.example.eu".into()),
            acme_email: Some("me@example.eu".into()),
            admin_password: Some("Abc123".into()),
            mail: true,
        })
        .unwrap();
        assert!(yaml.starts_with("#cloud-config\n"));
        assert_eq!(
            env_line(&yaml, "ARTCRAFT_DOMAIN"),
            "ARTCRAFT_DOMAIN=cloud.example.eu"
        );
        assert_eq!(
            env_line(&yaml, "ARTCRAFT_ACME_EMAIL"),
            "ARTCRAFT_ACME_EMAIL=me@example.eu"
        );
        assert_eq!(
            env_line(&yaml, "ARTCRAFT_ADMIN_PASSWORD"),
            "ARTCRAFT_ADMIN_PASSWORD=Abc123"
        );
        assert_eq!(env_line(&yaml, "ARTCRAFT_MAIL"), "ARTCRAFT_MAIL=1");
        // The comment block at the top documents the same keys; it must stay untouched.
        assert!(yaml.contains("#   ARTCRAFT_DOMAIN "));
    }

    #[test]
    fn empty_settings_keep_the_fallbacks() {
        let yaml = render(&Settings::default()).unwrap();
        assert_eq!(env_line(&yaml, "ARTCRAFT_DOMAIN"), "ARTCRAFT_DOMAIN=");
        assert_eq!(env_line(&yaml, "ARTCRAFT_MAIL"), "ARTCRAFT_MAIL=0");
    }

    #[test]
    fn refuses_values_that_could_break_the_yaml() {
        let bad = Settings {
            domain: Some("x.eu\n  - rm -rf /".into()),
            ..Default::default()
        };
        assert!(render(&bad).is_err());
        let bad = Settings {
            admin_password: Some("a b".into()),
            ..Default::default()
        };
        assert!(render(&bad).is_err());
    }

    #[test]
    fn passwords_are_long_and_alphanumeric() {
        let a = generate_password();
        assert_eq!(a.len(), 24);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(a, generate_password());
    }
}
