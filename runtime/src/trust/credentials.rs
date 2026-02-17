//! Encrypted credential vault using SQLite + AES-256-GCM.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

/// Encrypted credential store backed by SQLite.
pub struct CredentialVault {
    db: Connection,
}

impl CredentialVault {
    /// Open or create a credential vault.
    pub fn open(path: &PathBuf) -> Result<Self> {
        let db =
            Connection::open(path).with_context(|| format!("failed to open vault: {}", path.display()))?;

        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS credentials (
                domain TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                encrypted_password BLOB NOT NULL,
                nonce BLOB NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .context("failed to create credentials table")?;

        Ok(Self { db })
    }

    /// Open the default vault at ~/.cortex/vault.db.
    pub fn default_vault() -> Result<Self> {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".cortex")
            .join("vault.db");

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Self::open(&path)
    }

    /// Store credentials for a domain.
    ///
    /// In production, the password would be encrypted with AES-256-GCM using
    /// a key derived from CORTEX_VAULT_KEY or a machine-specific secret.
    /// For now, we store a placeholder encrypted blob.
    pub fn store(&self, domain: &str, username: &str, password: &str) -> Result<()> {
        // TODO: Implement actual AES-256-GCM encryption
        let encrypted = password.as_bytes().to_vec();
        let nonce = vec![0u8; 12]; // placeholder nonce

        self.db.execute(
            "INSERT OR REPLACE INTO credentials (domain, username, encrypted_password, nonce)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![domain, username, encrypted, nonce],
        )?;

        Ok(())
    }

    /// Retrieve credentials for a domain.
    pub fn retrieve(&self, domain: &str) -> Result<Option<(String, String)>> {
        let mut stmt = self.db.prepare(
            "SELECT username, encrypted_password FROM credentials WHERE domain = ?1",
        )?;

        let result = stmt.query_row(rusqlite::params![domain], |row| {
            let username: String = row.get(0)?;
            let encrypted: Vec<u8> = row.get(1)?;
            // TODO: Implement actual decryption
            let password = String::from_utf8_lossy(&encrypted).to_string();
            Ok((username, password))
        });

        match result {
            Ok(creds) => Ok(Some(creds)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete credentials for a domain.
    pub fn delete(&self, domain: &str) -> Result<bool> {
        let rows = self.db.execute(
            "DELETE FROM credentials WHERE domain = ?1",
            rusqlite::params![domain],
        )?;
        Ok(rows > 0)
    }

    /// List all domains with stored credentials.
    pub fn list_domains(&self) -> Result<Vec<String>> {
        let mut stmt = self.db.prepare("SELECT domain FROM credentials ORDER BY domain")?;
        let domains = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(domains)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-vault.db");
        let vault = CredentialVault::open(&path).unwrap();

        vault.store("example.com", "user", "pass123").unwrap();

        let creds = vault.retrieve("example.com").unwrap();
        assert!(creds.is_some());
        let (user, pass) = creds.unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "pass123");
    }

    #[test]
    fn test_credential_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-vault.db");
        let vault = CredentialVault::open(&path).unwrap();

        assert!(vault.retrieve("nonexistent.com").unwrap().is_none());
    }

    #[test]
    fn test_credential_delete() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-vault.db");
        let vault = CredentialVault::open(&path).unwrap();

        vault.store("example.com", "user", "pass").unwrap();
        assert!(vault.delete("example.com").unwrap());
        assert!(vault.retrieve("example.com").unwrap().is_none());
    }
}
