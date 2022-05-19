create table feature_names (
    id serial primary key,

    configuration_id integer not null,

    name text not null,

    foreign key (configuration_id) references configurations (id)
);

create index feature_names_configuration_id_idx on feature_names (configuration_id);
