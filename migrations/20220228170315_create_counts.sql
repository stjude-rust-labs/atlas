create table counts (
    id serial primary key,

    sample_id integer not null,
    annotation_id integer not null,

    data_type text not null,

    created_at timestamptz not null default now(),

    foreign key (sample_id) references samples (id)
);
