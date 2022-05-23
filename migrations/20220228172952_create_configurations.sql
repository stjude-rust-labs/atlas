create type strand_specification as enum ('none', 'forward', 'reverse');

create table configurations (
    id serial primary key,

    annotation_id integer not null,

    feature_type text not null,
    feature_name text not null,
    strand_specification strand_specification not null,

    created_at timestamptz not null default now(),

    foreign key (annotation_id) references annotations (id),
    unique (annotation_id, feature_type, feature_name)
);

create index configurations_annotation_id_idx on configurations (annotation_id);
