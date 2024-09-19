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
