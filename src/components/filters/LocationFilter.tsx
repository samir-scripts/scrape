import { Icon } from "../ui/Icon";
import { IconButton } from "../ui/IconButton";
import { useState, useEffect } from "react";
import { api } from "../../lib/api";
import {
  COUNTRIES,
  getRegionsForCountry,
  detectUserLocation,
} from "../../lib/locationData";
import { SearchCombobox, Select } from "../ui/Select";

export interface LocationFilterProps {
  countries: string[] | null;
  regions: string[] | null;
  remote: string | null;
  includeUnknown: boolean;
  onChange: (vals: {
    countries: string[] | null;
    regions: string[] | null;
    remote: string | null;
    includeUnknown: boolean;
  }) => void;
}

const REMOTE_OPTIONS: { id: string; label: string }[] = [
  { id: "all", label: "ANY WORKPLACE" },
  { id: "remote", label: "REMOTE ONLY" },
  { id: "hybrid", label: "HYBRID ONLY" },
  { id: "onsite", label: "ONSITE ONLY" },
];

export function LocationFilter({
  countries,
  regions,
  remote,
  includeUnknown,
  onChange,
}: LocationFilterProps) {
  const [dynamicCountries, setDynamicCountries] = useState<string[]>([]);
  const [countryQuery, setCountryQuery] = useState<string | null>(null);
  const [isDetecting, setIsDetecting] = useState(false);
  const [detectStatus, setDetectStatus] = useState<string | null>(null);

  useEffect(() => {
    api.listLocationOptions()
      .then((opts) => {
        if (opts.countries && opts.countries.length > 0) {
          const knownCodes = new Set(COUNTRIES.map((c) => c.code));
          const extra = opts.countries.filter((c) => !knownCodes.has(c.toUpperCase()));
          setDynamicCountries(extra);
        }
      })
      .catch(() => {
        // Fallback to static defaults
      });
  }, []);

  const selectedCountryCode =
    countries && countries.length > 0 ? countries[0].toUpperCase() : "ALL";

  const selectedRegions = regions || [];

  const cascadingRegions = getRegionsForCountry(selectedCountryCode);

  const handleCountryChange = (newCode: string) => {
    if (newCode === "ALL") {
      onChange({
        countries: null,
        regions,
        remote,
        includeUnknown,
      });
    } else {
      // Filter existing regions to only those relevant to the newly selected country
      const validForNewCountry = getRegionsForCountry(newCode).map((r) => r.code);
      const filteredRegions = selectedRegions.filter((reg) =>
        validForNewCountry.includes(reg.toUpperCase())
      );

      onChange({
        countries: [newCode],
        regions: filteredRegions.length > 0 ? filteredRegions : null,
        remote,
        includeUnknown,
      });
    }
  };

  const handleAddRegion = (regCode: string) => {
    if (!regCode) return;
    const upper = regCode.toUpperCase();
    if (!selectedRegions.includes(upper)) {
      onChange({
        countries,
        regions: [...selectedRegions, upper],
        remote,
        includeUnknown,
      });
    }
  };

  const handleRemoveRegion = (regCode: string) => {
    const next = selectedRegions.filter((r) => r !== regCode);
    onChange({
      countries,
      regions: next.length > 0 ? next : null,
      remote,
      includeUnknown,
    });
  };

  const handleClearRegions = () => {
    onChange({
      countries,
      regions: null,
      remote,
      includeUnknown,
    });
  };

  const handleAutoDetect = async () => {
    setIsDetecting(true);
    setDetectStatus(null);
    try {
      const loc = await detectUserLocation();
      if (loc.country) {
        const nextCountries = [loc.country];
        const nextRegions = loc.region ? [loc.region] : null;

        onChange({
          countries: nextCountries,
          regions: nextRegions,
          remote,
          includeUnknown,
        });

        const label = `${loc.country}${loc.region ? " · " + loc.region : ""}`;
        setDetectStatus(`DETECTED: ${label}`);
        setTimeout(() => setDetectStatus(null), 4000);
      } else {
        setDetectStatus("LOCATION UNAVAILABLE");
        setTimeout(() => setDetectStatus(null), 3000);
      }
    } catch {
      setDetectStatus("DETECTION ERROR");
      setTimeout(() => setDetectStatus(null), 3000);
    } finally {
      setIsDetecting(false);
    }
  };

  const hasFilter =
    (countries != null && countries.length > 0) ||
    (regions != null && regions.length > 0) ||
    (remote != null && remote !== "all");

  return (
    <div className="flex flex-col gap-3 font-mono text-xs select-none">
      {/* Header with Auto-Detect and Reset */}
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          GEOGRAPHY & REMOTE POLICY
        </label>
        <div className="flex items-center gap-2">
          <button
            type="button"
            data-control-action="filters.location.detect" onClick={handleAutoDetect}
            disabled={isDetecting}
            className="text-xs text-[var(--color-ink)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] border border-[var(--color-ink)] px-1.5 py-0.5 uppercase cursor-pointer disabled:opacity-50"
            title="Auto-detect system/user location"
          >
            {isDetecting ? "DETECTING..." : "📍 AUTO-DETECT"}
          </button>
          {hasFilter && (
            <button data-control-action="filters.location.clear"
              onClick={() =>
                onChange({
                  countries: null,
                  regions: null,
                  remote: "all",
                  includeUnknown,
                })
              }
              className="text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-danger)] uppercase cursor-pointer"
            >
              [RESET]
            </button>
          )}
        </div>
      </div>

      {detectStatus && (
        <div className="text-xs font-bold text-[var(--color-ink)] bg-[var(--color-paper-2)] border border-[var(--color-ink)] px-2 py-0.5 uppercase">
          {detectStatus}
        </div>
      )}

      {/* Workplace Mode (Remote / Hybrid / Onsite) */}
      <div className="flex flex-col gap-1">
        <span className="text-xs text-[var(--color-ink-mute)] uppercase font-bold">
          WORKPLACE POLICY
        </span>
        <div className="grid grid-cols-2 gap-1.5">
          {REMOTE_OPTIONS.map((opt) => {
            const active = (remote || "all") === opt.id;
            return (
              <button
                data-control-action="filters.location.preset" key={opt.id}
                type="button"
                onClick={() =>
                  onChange({
                    countries,
                    regions,
                    remote: opt.id === "all" ? null : opt.id,
                    includeUnknown,
                  })
                }
                className={`py-1 px-2 text-xs font-bold uppercase tracking-wider border cursor-pointer ${
                  active
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Target Country Dropdown */}
      <div className="flex flex-col gap-1 pt-1">
        <div className="flex justify-between items-center">
          <span className="text-xs text-[var(--color-ink-mute)] uppercase font-bold">
            COUNTRY
          </span>
          {selectedCountryCode !== "ALL" && (
            <button
              type="button"
              data-control-action="filters.location.country.clear" onClick={() => handleCountryChange("ALL")}
              className="text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-danger)] uppercase cursor-pointer"
            >
              [ANY COUNTRY]
            </button>
          )}
        </div>
        <SearchCombobox aria-label="Country" actionId="filters.location.country" value={selectedCountryCode}
          query={countryQuery ?? (selectedCountryCode === "ALL" ? "" : COUNTRIES.find(country => country.code === selectedCountryCode)?.name || selectedCountryCode)}
          onFocus={() => setCountryQuery("")} onBlur={() => setCountryQuery(null)} onQueryChange={setCountryQuery}
          onChange={code => { handleCountryChange(code); setCountryQuery(null); }}
          options={[
            { value: "ALL", label: "All countries / any region" },
            ...COUNTRIES.map((country) => ({ value: country.code, label: `${country.name} (${country.code})` })),
            ...dynamicCountries.map((country) => ({ value: country, label: country.toUpperCase() })),
          ]} />
      </div>

      {/* Cascading States & Provinces Dropdown */}
      <div className="flex flex-col gap-1 pt-1">
        <div className="flex justify-between items-center">
          <span className="text-xs text-[var(--color-ink-mute)] uppercase font-bold">
            STATES & PROVINCES
            {selectedCountryCode !== "ALL" ? ` (${selectedCountryCode})` : ""}
          </span>
          {selectedRegions.length > 0 && (
            <button
              type="button"
              data-control-action="filters.location.regions.clear" onClick={handleClearRegions}
              className="text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-danger)] uppercase cursor-pointer"
            >
              [CLEAR ({selectedRegions.length})]
            </button>
          )}
        </div>

        <Select aria-label="Add state/province" actionId="filters.location.region.add" value="" onChange={handleAddRegion}
          options={[{ value: "", label: "+ ADD STATE / PROVINCE..." }, ...cascadingRegions.map((region) => {
            const alreadySelected = selectedRegions.includes(region.code);
            return { value: region.code, label: `${region.name.toUpperCase()} (${region.code}) ${alreadySelected ? "✓" : ""}`, disabled: alreadySelected };
          })]} />

        {/* Selected State Tags / Chips */}
        {selectedRegions.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-1.5">
            {selectedRegions.map((reg) => {
              const matched = cascadingRegions.find(
                (r) => r.code === reg.toUpperCase()
              );
              const label = matched
                ? `${matched.name.toUpperCase()} (${reg})`
                : reg.toUpperCase();

              return (
                <span
                  key={reg}
                  className="inline-flex items-center gap-1.5 px-2 py-0.5 text-xs font-mono font-bold uppercase border border-[var(--color-ink)] bg-[var(--color-paper-2)] text-[var(--color-ink)]"
                >
                  <span>{label}</span>
                  <IconButton data-control-action="filters.location.region.remove" label={`Remove ${reg}`} onClick={() => handleRemoveRegion(reg)}>
                    <Icon name="close" size="sm" />
                  </IconButton>
                </span>
              );
            })}
          </div>
        )}
      </div>

      {/* Include Unknown Toggle */}
      <div className="flex items-center gap-2 pt-1">
        <label className="flex items-center gap-2 cursor-pointer text-xs text-[var(--color-ink)]">
          <input data-control-action="filters.location.unknown"
            type="checkbox"
            checked={includeUnknown}
            onChange={(e) =>
              onChange({
                countries,
                regions,
                remote,
                includeUnknown: e.target.checked,
              })
            }
            className="w-3.5 h-3.5 border border-[var(--color-ink)] bg-[var(--color-paper)] rounded-none accent-[var(--color-ink)] cursor-pointer"
          />
          <span className="uppercase font-bold tracking-wider">
            INCLUDE POSTINGS WITHOUT PARSED LOCATION
          </span>
        </label>
      </div>
    </div>
  );
}
