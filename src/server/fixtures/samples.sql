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
  (annotation_id, feature_type, feature_name, strand_specification)
values
  (1, 'exon', 'gene_name', 'reverse'),
  (2, 'exon', 'gene_name', 'reverse');

insert into runs
  (sample_id, configuration_id, data_type)
values
  (1, 1, 'RNA-Seq'),
  (1, 2, 'RNA-Seq'),
  (2, 1, 'RNA-Seq');
