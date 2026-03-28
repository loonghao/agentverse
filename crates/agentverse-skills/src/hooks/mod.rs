//! Hook-based publishing system for skill packages.
//!
//! When a skill version is published the API layer calls
//! `HookRegistry::run_all(record)`, which fires every registered hook in
//! sequence. The primary hook (`MetadataHook`) writes the `SkillPackage`
//! record into the database so it can later be fetched for download/deploy.
//!
//! New hooks (e.g. sending a webhook notification, pushing to a CDN) can be
//! added by implementing `PublishHook` and registering with `HookRegistry`.

use std::sync::Arc;

use agentverse_core::{repository::SkillPackageRepository, skill::SkillPackage};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::error::SkillError;

// ── PublishHook trait ─────────────────────────────────────────────────────────

/// A hook that is called when a skill package is published.
#[async_trait]
pub trait PublishHook: Send + Sync {
    /// Called with the package record that was just resolved at publish time.
    async fn on_publish(&self, pkg: &SkillPackage) -> Result<(), SkillError>;
}

// ── HookRegistry ─────────────────────────────────────────────────────────────

/// Runs all registered hooks in registration order.
///
/// Hooks are fire-and-forget: a failing hook logs a warning but does not abort
/// the publish operation (unless you change `run_all` to propagate).
#[derive(Default)]
pub struct HookRegistry {
    hooks: Vec<Arc<dyn PublishHook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn register(&mut self, hook: Arc<dyn PublishHook>) -> &mut Self {
        self.hooks.push(hook);
        self
    }

    /// Execute all hooks for the given package record.
    pub async fn run_all(&self, pkg: &SkillPackage) {
        for hook in &self.hooks {
            if let Err(e) = hook.on_publish(pkg).await {
                tracing::warn!(error = %e, "publish hook failed (non-fatal)");
            }
        }
    }
}

// ── MetadataHook ─────────────────────────────────────────────────────────────

/// Persists skill package metadata into the database.
///
/// This is the canonical hook that records the `SkillPackage` row so that
/// later download/deploy operations can look up the correct URL and checksum.
pub struct MetadataHook {
    repo: Arc<dyn SkillPackageRepository>,
}

impl MetadataHook {
    pub fn new(repo: Arc<dyn SkillPackageRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl PublishHook for MetadataHook {
    async fn on_publish(&self, pkg: &SkillPackage) -> Result<(), SkillError> {
        // Assign a fresh ID and timestamp if not already set (callers may
        // supply a pre-built record to make tests deterministic).
        let record = if pkg.id == Uuid::nil() {
            SkillPackage {
                id: Uuid::new_v4(),
                created_at: Utc::now(),
                ..pkg.clone()
            }
        } else {
            pkg.clone()
        };

        self.repo
            .register(record)
            .await
            .map(|_| ())
            .map_err(|e| SkillError::Hook(e.to_string()))
    }
}

// ── LoggingHook (diagnostic) ──────────────────────────────────────────────────

/// A simple hook that logs every publish event (useful for debugging).
pub struct LoggingHook;

#[async_trait]
impl PublishHook for LoggingHook {
    async fn on_publish(&self, pkg: &SkillPackage) -> Result<(), SkillError> {
        tracing::info!(
            version_id  = %pkg.artifact_version_id,
            source_type = %pkg.source_type,
            url         = %pkg.download_url,
            "skill package published",
        );
        Ok(())
    }
}
