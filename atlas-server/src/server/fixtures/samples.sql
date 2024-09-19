insert into samples
  (name, created_at)
values
  ('sample_1', '2022-02-18T21:05:05+00:00'),
  ('sample_2', '2022-02-18T21:05:06+00:00');

insert into annotations
  (name, genome_build)
values
  ('GENCODE 39', 'GRCh38.p13'),
  ('GENCODE 19', 'GRCh37.p13');

insert into configurations
  (annotation_id, feature_type, feature_name)
values
  (1, 'exon', 'gene_name'),
  (2, 'exon', 'gene_name');

insert into runs
  (sample_id, configuration_id, strand_specification, data_type)
values
  (1, 1, 'reverse', 'RNA-Seq'),
  (1, 2, 'reverse', 'RNA-Seq'),
  (2, 1, 'reverse', 'RNA-Seq');
