create table annotations (
    id serial primary key,

    name text not null,
    genome_build text not null,

    created_at timestamptz not null default now(),

    unique(name)

)
