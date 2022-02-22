insert into samples
    (name)
values
    ('sample_1'),
    ('sample_2');

insert into counts
    (sample_id, genome_build, gene_model, data_type)
values
    (1, 'GRCh38.p13', 'GENCODE 39', 'RNA-Seq'),
    (1, 'GRCh37.p13', 'GENCODE 19', 'RNA-Seq'),
    (2, 'GRCh38.p13', 'GENCODE 39', 'RNA-Seq');
