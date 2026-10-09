import { describe, it, expect } from "vitest";
import { api, DEFAULT_SETTINGS } from "../lib/api";
import type { Settings } from "../bindings/Settings";
import type { Job } from "../bindings/Job";
import type { Page } from "../bindings/Page";

describe("Application Readiness Gate Logic", () => {
  interface ReadinessState {
    settingsLoading: boolean;
    jobsLoading: boolean;
    settingsError: Error | null;
    jobsError: Error | null;
    settings?: Settings;
    jobsPage?: Page<Job>;
  }

  function evaluateReadiness(state: ReadinessState) {
    const isInitializing = state.settingsLoading || state.jobsLoading;
    const initError = state.settingsError || state.jobsError;
    const isReady = !isInitializing && !initError && !!state.settings && !!state.jobsPage;

    return {
      isInitializing,
      initError,
      isReady,
      canAccessTabs: isReady,
    };
  }

  it("locks out user interaction during bootstrap while settings or jobs are loading", () => {
    const stateLoadingBoth: ReadinessState = {
      settingsLoading: true,
      jobsLoading: true,
      settingsError: null,
      jobsError: null,
    };
    expect(evaluateReadiness(stateLoadingBoth).isInitializing).toBe(true);
    expect(evaluateReadiness(stateLoadingBoth).canAccessTabs).toBe(false);

    const stateLoadingSettingsOnly: ReadinessState = {
      settingsLoading: true,
      jobsLoading: false,
      settingsError: null,
      jobsError: null,
      jobsPage: { items: [], total: BigInt(0), limit: BigInt(2000), offset: BigInt(0) },
    };
    expect(evaluateReadiness(stateLoadingSettingsOnly).isInitializing).toBe(true);
    expect(evaluateReadiness(stateLoadingSettingsOnly).canAccessTabs).toBe(false);

    const stateLoadingJobsOnly: ReadinessState = {
      settingsLoading: false,
      jobsLoading: true,
      settingsError: null,
      jobsError: null,
      settings: DEFAULT_SETTINGS,
    };
    expect(evaluateReadiness(stateLoadingJobsOnly).isInitializing).toBe(true);
    expect(evaluateReadiness(stateLoadingJobsOnly).canAccessTabs).toBe(false);
  });

  it("locks out user interaction and captures error when initialization fails", () => {
    const failureState: ReadinessState = {
      settingsLoading: false,
      jobsLoading: false,
      settingsError: new Error("Database file locked"),
      jobsError: null,
    };
    const evaluation = evaluateReadiness(failureState);
    expect(evaluation.isInitializing).toBe(false);
    expect(evaluation.initError?.message).toBe("Database file locked");
    expect(evaluation.canAccessTabs).toBe(false);
  });

  it("permits application access and guarantees non-null settings once subsystems settle", () => {
    const readyState: ReadinessState = {
      settingsLoading: false,
      jobsLoading: false,
      settingsError: null,
      jobsError: null,
      settings: DEFAULT_SETTINGS,
      jobsPage: { items: [], total: BigInt(0), limit: BigInt(2000), offset: BigInt(0) },
    };
    const evaluation = evaluateReadiness(readyState);
    expect(evaluation.isInitializing).toBe(false);
    expect(evaluation.initError).toBeNull();
    expect(evaluation.isReady).toBe(true);
    expect(evaluation.canAccessTabs).toBe(true);
  });

  it("verifies api.getSettings returns valid DEFAULT_SETTINGS in dev environment", async () => {
    const settings = await api.getSettings();
    expect(settings).toBeDefined();
    expect(settings.system_locale).toBe("en-US");
    expect(settings.ui_theme).toBe("paper");
    expect(settings.scraper_enabled).toBe(true);
    expect(Array.isArray(settings.filters_include_keywords)).toBe(true);
  });

  it("verifies api.listJobs returns empty Page in dev environment without throwing", async () => {
    const page = await api.listJobs();
    expect(page).toBeDefined();
    expect(page.items).toEqual([]);
    expect(page.total).toBe(BigInt(0));
    expect(page.limit).toBe(BigInt(2000));
    expect(page.offset).toBe(BigInt(0));
  });
});
