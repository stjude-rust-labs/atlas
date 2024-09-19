create table samples (
    id serial primary key,
    name text not null,
    created_at timestamptz not null default now(),

    unique(name)
);

create table annotations (
    id serial primary key,

    name text not null,
    genome_build text not null,

    created_at timestamptz not null default now(),

    unique(name)
);

create table configurations (
    id serial primary key,

    annotation_id integer not null,

    feature_type text not null,
    feature_name text not null,

    created_at timestamptz not null default now(),

    foreign key (annotation_id) references annotations (id),
    unique (annotation_id, feature_type, feature_name)
);

create index configurations_annotation_id_idx on configurations (annotation_id);

create table features (
    id serial primary key,

    configuration_id integer not null,

    name text not null,
    length integer not null,

    foreign key (configuration_id) references configurations (id),
    unique (configuration_id, name)
);

create type strand_specification as enum ('none', 'forward', 'reverse');

create table runs (
    id serial primary key,

    sample_id integer not null,
    configuration_id integer not null,

    strand_specification strand_specification not null,
    data_type text not null,

    created_at timestamptz not null default now(),

    foreign key (sample_id) references samples (id),
    foreign key (configuration_id) references configurations (id)
);

create index runs_sample_id_idx on runs (sample_id);
create index runs_configuration_id_idx on runs (configuration_id);

create table counts (
    id serial primary key,

    run_id integer not null,
    feature_id integer not null,

    value integer not null,

    foreign key (run_id) references runs (id),
    foreign key (feature_id) references features (id)
);

create index counts_run_id_feature_id_idx on counts(run_id, feature_id);

create table datasets (
    id serial primary key,
    name text not null,
    created_at timestamptz not null default now(),

    unique(name)
);

create table datasets_runs (
    id serial primary key,

    dataset_id integer not null,
    run_id integer not null,

    foreign key (dataset_id) references datasets (id),
    foreign key (run_id) references runs (id)
);

create index counts_dataset_id_run_id_idx on datasets_runs(dataset_id, run_id);

create type status as enum ('queued', 'running', 'success', 'failed');

create table tasks (
    id uuid primary key,

    status status not null,
    message jsonb not null,

    created_at timestamptz not null default now()
);

create table results (
    id uuid primary key,

    body jsonb not null,

    created_at timestamptz not null default now()
);
