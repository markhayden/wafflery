-- Existing examples remain readable. Missing historical input occurrences never claim portability.
UPDATE examples SET record=json_set(record,
    '$.input_artifacts',json('[]'),
    '$.workspace_dependent',json(CASE WHEN EXISTS(
        SELECT 1 FROM json_tree(record,'$.request.bindings') WHERE key='kind' AND value='artifact'
    ) THEN 'true' ELSE 'false' END));
