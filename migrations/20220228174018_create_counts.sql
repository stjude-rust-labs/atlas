create table counts (
    id serial primary key,

    sample_id integer not null,
    configuration_id integer not null,

    data_type text not null,

    created_at timestamptz not null default now(),

    foreign key (sample_id) references samples (id),
    foreign key (configuration_id) references configurations (id)
);
