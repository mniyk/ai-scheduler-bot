// src/store/sqlx_store.rs

use async_trait::async_trait;

use crate::domain::{Event, EventStatus, NewEvent};
use crate::error::{AppError, AppResult};
use super::EventStore;
use chrono::{DateTime, Utc};

#[cfg(feature = "sqlite")]
pub type DbPool = sqlx::SqlitePool;
#[cfg(feature = "postgres")]
pub type DbPool = sqlx::PgPool;

#[cfg(all(feature = "sqlite", feature = "postgres"))]
compile_error!("Enable only one of `sqlite` or `postgres` features.");

#[cfg(not(any(feature = "sqlite", feature = "postgres")))]
compile_error!("Enable one of `sqlite` or `postgres` features.");

pub struct SqlxStore {
    pool: DbPool,
}

impl SqlxStore {
    pub async fn connect(database_url: &str) -> AppResult<Self> {
        #[cfg(feature = "sqlite")]
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        #[cfg(feature = "postgres")]
        let pool = sqlx::PgPool::connect(database_url).await?;

        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> AppResult<()> {
        #[cfg(feature = "sqlite")]
        sqlx::migrate!("./migrations/sqlite").run(&self.pool).await
            .map_err(|e| AppError::Internal(format!("migration failed: {}", e)))?;
        #[cfg(feature = "postgres")]
        sqlx::migrate!("./migrations/postgres").run(&self.pool).await
            .map_err(|e| AppError::Internal(format!("migration failed: {}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl EventStore for SqlxStore {
    async fn create(&self, new_event: NewEvent) -> AppResult<Event> {
        let s = &new_event.schedule;
        let related_urls_json = serde_json::to_string(&s.related_urls)?;
        let participants_json = serde_json::to_string(&s.participants)?;

        sqlx::query(
            r#"
            INSERT INTO events (
                id, slack_user_id, slack_team_id, title,
                start_at, end_at, timezone,
                location, meeting_url, related_urls, participants,
                notes, reminder_at, recurrence, tag, status
            ) VALUES (
                $1, $2, $3, $4,
                $5, $6, $7,
                $8, $9, $10, $11,
                $12, $13, $14, $15, $16
            )
            "#,
        )
        .bind(&new_event.id)
        .bind(&new_event.slack_user_id)
        .bind(&new_event.slack_team_id)
        .bind(&s.title)
        .bind(s.start_at)
        .bind(s.end_at)
        .bind(&s.timezone)
        .bind(&s.location)
        .bind(&s.meeting_url)
        .bind(&related_urls_json)
        .bind(&participants_json)
        .bind(&s.notes)
        .bind(s.reminder_at)
        .bind(&s.recurrence)
        .bind(&s.tag)
        .bind(EventStatus::Active.as_str())
        .execute(&self.pool)
        .await?;

        self.get(&new_event.id)
            .await?
            .ok_or_else(|| AppError::Internal("event missing after insert".into()))
    }

    async fn get(&self, id: &str) -> AppResult<Option<Event>> {
        let row: Option<EventRow> = sqlx::query_as::<_, EventRow>(
            r#"
            SELECT id, slack_user_id, slack_team_id, title,
                   start_at, end_at, timezone,
                   location, meeting_url, related_urls, participants,
                   notes, reminder_at, recurrence, tag, status,
                   created_at, updated_at
              FROM events
             WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Event::try_from).transpose()
    }

    async fn cancel(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query(
            r#"UPDATE events SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP WHERE id = $1"#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::EventNotFound(id.to_string()));
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: String,
    slack_user_id: String,
    slack_team_id: String,
    title: String,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    timezone: String,
    location: Option<String>,
    meeting_url: Option<String>,
    related_urls: Option<String>,
    participants: Option<String>,
    notes: Option<String>,
    reminder_at: Option<DateTime<Utc>>,
    recurrence: Option<String>,
    tag: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<EventRow> for Event {
    type Error = AppError;

    fn try_from(r: EventRow) -> Result<Self, Self::Error> {
        let related_urls: Vec<String> = match r.related_urls {
            Some(s) if !s.is_empty() => serde_json::from_str(&s)?,
            _ => Vec::new(),
        };
        let participants: Vec<String> = match r.participants {
            Some(s) if !s.is_empty() => serde_json::from_str(&s)?,
            _ => Vec::new(),
        };
        let status = EventStatus::from_str(&r.status)
            .ok_or_else(|| AppError::Internal(format!("invalid status: {}", r.status)))?;

        Ok(Event {
            id: r.id,
            slack_user_id: r.slack_user_id,
            slack_team_id: r.slack_team_id,
            title: r.title,
            start_at: r.start_at,
            end_at: r.end_at,
            timezone: r.timezone,
            location: r.location,
            meeting_url: r.meeting_url,
            related_urls,
            participants,
            notes: r.notes,
            reminder_at: r.reminder_at,
            recurrence: r.recurrence,
            tag: r.tag,
            status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}