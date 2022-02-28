create table counts (
    id serial primary key,

    run_id integer not null,
    feature_name_id integer not null,

    value integer not null,

    foreign key (run_id) references runs (id),
    foreign key (feature_name_id) references feature_names (id)
);
