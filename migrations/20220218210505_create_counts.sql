create table counts (
    id serial primary key,

    sample_id integer not null,

    genome_build text not null,
    gene_model text not null,
    data_type text not null,

    created_at timestamptz not null default now(),

    foreign key (sample_id) references samples (id)
);
