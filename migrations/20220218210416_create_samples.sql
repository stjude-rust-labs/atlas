create table samples (
    id serial primary key,
    name text not null,
    created_at timestamptz not null default now(),

    unique(name)
);

