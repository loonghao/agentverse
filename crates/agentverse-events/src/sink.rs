//! EventSink trait — the port for persisting domain events.
//!
//! Separating the trait from the concrete `EventStore` implementation lets
//! tests swap in a no-op sink without requiring a real database connection.

use async_trait::async_trait;
use uuid::Uuid;

use agentverse_core::error::CoreError;

use crate::types::{DomainEvent, EventEnvelope};

/// Port for appending and replaying domain events.
#[async_trait]
pub trait EventSink: Send + Sync {
    /// Append a domain event and return the stored envelope.
    async fn append(&self, event: DomainEvent) -> Result<EventEnvelope, CoreError>;

    /// Load all events for an aggregate, ordered by sequence.
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<EventEnvelope>, CoreError>;
}

// ── No-op implementation for unit / integration tests ─────────────────────────

/// A no-op event sink that discards every event without touching a database.
/// Use this in tests to isolate handlers from persistence concerns.
pub struct NoopEventSink;

#[async_trait]
impl EventSink for NoopEventSink {
    async fn append(&self, event: DomainEvent) -> Result<EventEnvelope, CoreError> {
        let aggregate_id = event.aggregate_id();
        Ok(EventEnvelope {
            id: uuid::Uuid::new_v4(),
            aggregate_type: event.aggregate_type().to_string(),
            aggregate_id,
            event_type: event.event_type().to_string(),
            payload: serde_json::to_value(&event).unwrap_or(serde_json::Value::Null),
            sequence: 1,
            occurred_at: chrono::Utc::now(),
        })
    }

    async fn load(&self, _aggregate_id: Uuid) -> Result<Vec<EventEnvelope>, CoreError> {
        Ok(vec![])
    }
}
