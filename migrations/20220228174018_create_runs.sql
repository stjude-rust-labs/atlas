create table runs (
    id serial primary key,

    sample_id integer not null,
    configuration_id integer not null,

    data_type text not null,

    created_at timestamptz not null default now(),

    foreign key (sample_id) references samples (id),
    foreign key (configuration_id) references configurations (id)
);

create index runs_sample_id_idx on runs (sample_id);
create index runs_configuration_id_idx on runs (configuration_id);
