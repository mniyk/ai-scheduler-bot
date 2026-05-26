-- migrations/sqlite/0001_init.sql

CREATE TABLE events (
    id              TEXT PRIMARY KEY,
    slack_user_id   TEXT NOT NULL,
    slack_team_id   TEXT NOT NULL,
    title           TEXT NOT NULL,
    start_at        TIMESTAMP NOT NULL,
    end_at          TIMESTAMP NOT NULL,
    timezone        TEXT NOT NULL,
    location        TEXT,
    meeting_url     TEXT,
    related_urls    TEXT,
    participants    TEXT,
    notes           TEXT,
    reminder_at     TIMESTAMP,
    recurrence      TEXT,
    tag             TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_events_user_start ON events(slack_user_id, start_at);
CREATE INDEX idx_events_status ON events(status);
CREATE INDEX idx_events_reminder ON events(reminder_at) WHERE reminder_at IS NOT NULL;