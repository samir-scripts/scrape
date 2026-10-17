ALTER TABLE employer_branding ADD COLUMN asset_data TEXT;
UPDATE employer_branding
SET asset_data = COALESCE(asset_data, json_extract(logo_json, '$.asset')),
    logo_json = CASE WHEN json_extract(logo_json, '$.asset') IS NOT NULL THEN json_remove(logo_json, '$.asset') ELSE logo_json END
WHERE asset_data IS NULL AND json_extract(logo_json, '$.asset') IS NOT NULL;
