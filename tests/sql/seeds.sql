insert into samples
    (name)
values
    ('sample_1'),
    ('sample_2');

insert into annotations
    (name, genome_build)
values
    ('GENCODE 39', 'GRCh38.p13'),
    ('GENCODE 19', 'GRCh37.p13');

insert into configurations
    (annotation_id, feature_type, feature_name)
values
    (1, 'exon', 'gene_name');

insert into counts
    (sample_id, annotation_id, data_type)
values
    (1, 1, 'RNA-Seq'),
    (1, 2, 'RNA-Seq'),
    (2, 1, 'RNA-Seq');
