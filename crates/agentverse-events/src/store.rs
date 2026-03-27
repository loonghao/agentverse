use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use agentverse_core::error::{CoreError, StorageError};

use crate::types::{DomainEvent, EventEnvelope};

// Re-use the event entity from storage by defining a minimal inline entity.
// This avoids a circular dependency between agentverse-events and agentverse-storage.
mod event_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub aggregate_type: String,
        pub aggregate_id: Uuid,
        pub event_type: String,
        pub payload: Json,
        pub sequence: i64,
        pub occurred_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Append-only event store backed by PostgreSQL.
pub struct EventStore {
    db: DatabaseConnection,
}

impl EventStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Append a domain event to the store.
    /// Returns the stored envelope with its assigned sequence number.
    pub async fn append(&self, event: DomainEvent) -> Result<EventEnvelope, CoreError> {
        let aggregate_id = event.aggregate_id();
        let next_seq = self.next_sequence(aggregate_id).await?;

        let payload = serde_json::to_value(&event)
            .map_err(|e| CoreError::Internal(e.to_string()))?;

        let now = Utc::now().fixed_offset();
        let id = Uuid::new_v4();

        let model = event_entity::ActiveModel {
            id: Set(id),
            aggregate_type: Set(event.aggregate_type().to_string()),
            aggregate_id: Set(aggregate_id),
            event_type: Set(event.event_type().to_string()),
            payload: Set(payload.clone()),
            sequence: Set(next_seq),
            occurred_at: Set(now),
        };

        model
            .insert(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?;

        Ok(EventEnvelope {
            id,
            aggregate_type: event.aggregate_type().to_string(),
            aggregate_id,
            event_type: event.event_type().to_string(),
            payload,
            sequence: next_seq,
            occurred_at: now.with_timezone(&Utc),
        })
    }

    /// Fetch all events for a given aggregate (sorted by sequence).
    pub async fn load(&self, aggregate_id: Uuid) -> Result<Vec<EventEnvelope>, CoreError> {
        event_entity::Entity::find()
            .filter(event_entity::Column::AggregateId.eq(aggregate_id))
            .order_by_asc(event_entity::Column::Sequence)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))
            .map(|rows| {
                rows.into_iter()
                    .map(|m| EventEnvelope {
                        id: m.id,
                        aggregate_type: m.aggregate_type,
                        aggregate_id: m.aggregate_id,
                        event_type: m.event_type,
                        payload: m.payload,
                        sequence: m.sequence,
                        occurred_at: m.occurred_at.with_timezone(&Utc),
                    })
                    .collect()
            })
    }

    /// Next sequence number for this aggregate (max + 1, or 1 if none).
    async fn next_sequence(&self, aggregate_id: Uuid) -> Result<i64, CoreError> {
        use sea_orm::QuerySelect;

        #[derive(sea_orm::FromQueryResult)]
        struct MaxSeq {
            max_seq: Option<i64>,
        }

        let result = event_entity::Entity::find()
            .filter(event_entity::Column::AggregateId.eq(aggregate_id))
            .select_only()
            .column_as(event_entity::Column::Sequence.max(), "max_seq")
            .into_model::<MaxSeq>()
            .one(&self.db)
            .await
            .map_err(|e| CoreError::Storage(StorageError(e.to_string())))?;

        Ok(result.and_then(|r| r.max_seq).unwrap_or(0) + 1)
    }
}

