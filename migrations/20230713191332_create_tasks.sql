create type status as enum ('queued', 'running', 'success', 'failed');

create table tasks (
    id uuid primary key,

    status status not null,
    message jsonb not null,

    created_at timestamptz not null default now()
)
