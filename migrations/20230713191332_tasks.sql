create table tasks (
    id uuid primary key,

    status smallint not null,
    message jsonb not null,

    created_at timestamptz not null default now()
)
