-- Migration 0008: Add company_icon_url support to jobs and sources

ALTER TABLE jobs ADD COLUMN company_icon_url TEXT;
ALTER TABLE sources ADD COLUMN company_icon_url TEXT;
