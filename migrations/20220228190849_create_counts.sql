create table counts (
    id serial primary key,

    run_id integer not null,
    feature_id integer not null,

    value integer not null,

    foreign key (run_id) references runs (id),
    foreign key (feature_id) references features (id)
);

create index counts_run_id_feature_id_idx on counts(run_id, feature_id);
