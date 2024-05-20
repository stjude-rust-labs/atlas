insert into annotations
  (name, genome_build)
values
  ('GENCODE 40', 'GRCh38.p13');

insert into configurations
  (annotation_id, feature_type, feature_name)
values
  (1, 'gene', 'gene_name');
