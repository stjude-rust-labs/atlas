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

insert into features
  (configuration_id, name, length)
values
  (1, '39_feature_1', 8),
  (1, '39_feature_2', 13),
  (2, '19_feature_1', 8),
  (2, '19_feature_2', 13);

insert into samples
  (name)
values
  ('sample1'),
  ('sample2');

insert into runs
  (sample_id, configuration_id, strand_specification, data_type)
values
  (1, 1, 'reverse', 'RNA-Seq'),
  (2, 1, 'reverse', 'RNA-Seq');

insert into counts
  (run_id, feature_id, value)
values
  (1, 1, 5),
  (1, 2, 8),
  (2, 1, 13),
  (2, 2, 21);
