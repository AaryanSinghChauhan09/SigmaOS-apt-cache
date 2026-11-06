// SPDX-License-Identifier: Apache-2.0
//! SigmaOS-apt-cache — apt-cache-style package metadata query, Debian-inspired.
//!
//! Models the read side of APT: an in-memory cache of package records that
//! supports `search` (substring match on name/description) and `show`
//! (exact record lookup). Fail-fast on unknown packages.

use std::fmt;

/// Errors querying the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// No package record with the requested name.
    NotFound(String),
    /// Empty package name or version supplied.
    EmptyField(&'static str),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(n) => write!(f, "package not found: {n}"),
            Self::EmptyField(field) => write!(f, "empty field: {field}"),
        }
    }
}

impl std::error::Error for CacheError {}

/// A single binary package record, mirroring apt-cache `show` output fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRecord {
    /// Binary package name.
    pub name: String,
    /// Candidate version.
    pub version: String,
    /// Short description.
    pub description: String,
    /// Source package name, if different.
    pub source: Option<String>,
}

impl PackageRecord {
    /// Create a record; fails fast on empty required fields.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, CacheError> {
        let (name, version) = (name.into(), version.into());
        if name.trim().is_empty() {
            return Err(CacheError::EmptyField("name"));
        }
        if version.trim().is_empty() {
            return Err(CacheError::EmptyField("version"));
        }
        Ok(Self {
            name,
            version,
            description: description.into(),
            source: None,
        })
    }
}

/// In-memory package cache. Read-heavy, write-once-at-load (SRP, immutable queries).
#[derive(Debug, Default)]
pub struct PackageCache {
    packages: Vec<PackageRecord>,
}

impl PackageCache {
    /// Empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load records (e.g. parsed from package lists).
    pub fn load(&mut self, records: Vec<PackageRecord>) {
        self.packages = records;
    }

    /// `apt-cache search <query>` — case-insensitive substring over name+description.
    pub fn search(&self, query: &str) -> Vec<&PackageRecord> {
        let q = query.to_lowercase();
        self.packages
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// `apt-cache show <pkg>` — exact name lookup, fail-fast if absent.
    pub fn show(&self, name: &str) -> Result<&PackageRecord, CacheError> {
        self.packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| CacheError::NotFound(name.to_string()))
    }

    /// Number of cached records.
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache() -> PackageCache {
        let mut c = PackageCache::new();
        c.load(vec![
            PackageRecord::new("bash", "5.2.15-1", "GNU Bourne Again shell").unwrap(),
            PackageRecord::new("coreutils", "9.4-1", "GNU core utilities").unwrap(),
            PackageRecord::new("apt", "2.6.1", "commandline package manager").unwrap(),
        ]);
        c
    }

    #[test]
    fn search_matches_name() {
        let c = cache();
        let hits = c.search("bash");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "bash");
    }

    #[test]
    fn search_matches_description_case_insensitive() {
        assert_eq!(cache().search("SHELL").len(), 1);
    }

    #[test]
    fn show_returns_record() {
        assert_eq!(cache().show("apt").unwrap().version, "2.6.1");
    }

    #[test]
    fn show_missing_fails() {
        assert_eq!(
            cache().show("nonexistent"),
            Err(CacheError::NotFound("nonexistent".into()))
        );
    }

    #[test]
    fn empty_fields_rejected() {
        assert!(PackageRecord::new("", "1.0", "d").is_err());
        assert!(PackageRecord::new("n", "  ", "d").is_err());
    }
}
