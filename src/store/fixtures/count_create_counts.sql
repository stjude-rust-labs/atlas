insert into annotations
  (name, genome_build)
values
  ('GENCODE 40', 'GRCh38.p13');

insert into configurations
  (annotation_id, feature_type, feature_name)
values
  (1, 'gene', 'gene_name');

insert into samples (name) values ('sample1');

insert into runs
  (sample_id, configuration_id, strand_specification, data_type)
values
  (1, 1, 'reverse', 'RNA-Seq');

insert into features
  (configuration_id, name)
values
  (1, 'feature1'),
  (1, 'feature2');
