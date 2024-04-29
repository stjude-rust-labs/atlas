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

insert into features
  (configuration_id, name)
values
  (1, '39_feature_1'),
  (1, '39_feature_2'),
  (2, '19_feature_1'),
  (2, '19_feature_2');

insert into samples
  (name)
values
  ('sample1'),
  ('sample2');

insert into runs
  (sample_id, configuration_id, data_type)
values
  (1, 1, 'RNA-Seq'),
  (2, 1, 'RNA-Seq');

insert into counts
  (run_id, feature_id, value)
values
  (1, 1, 5),
  (1, 2, 8),
  (2, 1, 13),
  (2, 2, 21);
