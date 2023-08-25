create table results (
    id uuid primary key,

    body jsonb not null,

    created_at timestamptz not null default now()
)
