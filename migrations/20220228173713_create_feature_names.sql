create table feature_names (
    id serial primary key,

    configuration_id integer not null,

    name text not null,

    foreign key (configuration_id) references configurations (id)
)
