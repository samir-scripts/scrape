-- Legacy "all" must not opt tracked or annotated jobs into deletion.
UPDATE sources SET retention_policy = 'untracked_only' WHERE retention_policy = 'all';
