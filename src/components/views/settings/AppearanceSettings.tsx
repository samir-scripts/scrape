import type { Settings } from "../../../bindings/Settings";
import { Toggle } from "../../ui/Toggle";
import { Rule } from "../../ui/Rule";
import { useUIStore } from "../../../lib/store";
import { FONT_OPTIONS, type FontFamilyId } from "../../../lib/constants";
import { Select } from "../../ui/Select";

interface AppearanceSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function AppearanceSettings({
  settings,
  onUpdateSetting,
}: AppearanceSettingsProps) {
  const currentTheme = settings.ui_theme || "paper";
  const currentFont = (settings.ui_font_family as FontFamilyId) || "default";

  const handleSelectTheme = (theme: "paper" | "parchment" | "obsidian") => {
    onUpdateSetting("ui.theme", theme);
    document.documentElement.setAttribute("data-theme", theme);
  };

  const handleSelectFont = (fontId: FontFamilyId) => {
    onUpdateSetting("ui.font_family", fontId);
  };

  const handleToggleTooltips = (enabled: boolean) => {
    onUpdateSetting("ui.tooltips_enabled", enabled);
    useUIStore.getState().setTooltipsEnabled(enabled);
  };

  const handleToggleReduceMotion = (reduced: boolean) => {
    onUpdateSetting("ui.reduce_motion", reduced);
    document.documentElement.setAttribute(
      "data-reduce-motion",
      reduced ? "true" : "false",
    );
  };

  return (
    <div className="flex flex-col gap-6 w-full max-w-3xl select-none">
      <div>
        <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
          APPEARANCE
        </h2>
      </div>

      <Rule />

      {/* Theme Cards */}
      <div className="flex flex-col gap-3 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          THEME
        </label>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {/* Paper Theme Card */}
          <div
            role="button"
            tabIndex={0}
            onClick={() => handleSelectTheme("paper")}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleSelectTheme("paper");
              }
            }}
            className={`p-4 border-2 cursor-pointer flex flex-col gap-3 bg-[#CEC8B8] text-[#3F3F37] focus:outline-none ${
              currentTheme === "paper"
                ? "border-[#3F3F37]"
                : "border-[#6C6D5F]/50"
            }`}
          >
            <div className="flex justify-between items-center font-bold">
              <span>PAPER</span>
              {currentTheme === "paper" && (
                <span className="bg-[#3F3F37] text-[#CEC8B8] px-1.5 py-0.5 text-xs">
                  ACTIVE
                </span>
              )}
            </div>
            <p className="text-xs text-[#535448]">
              Muted warm gray-beige inspired by NieR.
            </p>
            <div className="flex gap-1.5 pt-2 border-t border-[#3F3F37]">
              <span className="w-5 h-5 bg-[#CEC8B8] border border-[#3F3F37]" title="Surface" />
              <span className="w-5 h-5 bg-[#D9D3C3] border border-[#3F3F37]" title="Raised" />
              <span className="w-5 h-5 bg-[#ECE7DB] border border-[#3F3F37]" title="Card" />
              <span className="w-5 h-5 bg-[#3F3F37]" title="Ink" />
              <span className="w-5 h-5 bg-[#B9B5A5]" title="Selection" />
              <span className="w-5 h-5 bg-[#D86072]" title="Cherry" />
            </div>
          </div>

          {/* Parchment Theme Card (Classic) */}
          <div
            role="button"
            tabIndex={0}
            onClick={() => handleSelectTheme("parchment")}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleSelectTheme("parchment");
              }
            }}
            className={`p-4 border-2 cursor-pointer flex flex-col gap-3 bg-[#F5F5DC] text-[#1A1A1A] focus:outline-none ${
              currentTheme === "parchment"
                ? "border-[#1A1A1A]"
                : "border-[#1A1A1A]/40"
            }`}
          >
            <div className="flex justify-between items-center font-bold">
              <span>PARCHMENT</span>
              {currentTheme === "parchment" && (
                <span className="bg-[#1A1A1A] text-[#F5F5DC] px-1.5 py-0.5 text-xs">
                  ACTIVE
                </span>
              )}
            </div>
            <p className="text-xs text-[#676357]">
              Classic warm beige with high-contrast ink rules.
            </p>
            <div className="flex gap-1.5 pt-2 border-t border-[#1A1A1A]">
              <span className="w-5 h-5 bg-[#F5F5DC] border border-[#1A1A1A]" title="Surface" />
              <span className="w-5 h-5 bg-[#E6E2C8] border border-[#1A1A1A]" title="Raised" />
              <span className="w-5 h-5 bg-[#1A1A1A]" title="Ink" />
              <span className="w-5 h-5 bg-[#C6C2F2]" title="Lavender" />
              <span className="w-5 h-5 bg-[#D6DC82]" title="Lime" />
              <span className="w-5 h-5 bg-[#D86072]" title="Cherry" />
            </div>
          </div>

          {/* Obsidian Theme Card */}
          <div
            role="button"
            tabIndex={0}
            onClick={() => handleSelectTheme("obsidian")}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleSelectTheme("obsidian");
              }
            }}
            className={`p-4 border-2 cursor-pointer flex flex-col gap-3 bg-[#262628] text-[#FFF6EE] focus:outline-none ${
              currentTheme === "obsidian"
                ? "border-[var(--color-ink)]"
                : "border-[var(--color-ink-mute)]/50"
            }`}
          >
            <div className="flex justify-between items-center font-bold">
              <span>OBSIDIAN</span>
              {currentTheme === "obsidian" && (
                <span className="bg-[#FFF6EE] text-[#262628] px-1.5 py-0.5 text-xs">
                  ACTIVE
                </span>
              )}
            </div>
            <p className="text-xs text-[#B9B5AF]">
              Dark background with high-contrast text.
            </p>
            <div className="flex gap-1.5 pt-2 border-t border-[#FFF6EE]">
              <span className="w-5 h-5 bg-[#262628] border border-[#FFF6EE]" title="Surface" />
              <span className="w-5 h-5 bg-[#36363a] border border-[#FFF6EE]" title="Raised" />
              <span className="w-5 h-5 bg-[#FFF6EE]" title="Text" />
              <span className="w-5 h-5 bg-[#C6C2F2]" title="Lavender" />
              <span className="w-5 h-5 bg-[#BBD05D]" title="Lime" />
              <span className="w-5 h-5 bg-[#E87585]" title="Cherry" />
            </div>
          </div>
        </div>
      </div>

      <Rule />

      <section className="grid gap-4" aria-label="Typography">
        <label className="grid gap-2 text-sm">
          Interface font
          <Select aria-label="Interface font" value={currentFont} onChange={font => handleSelectFont(font as FontFamilyId)}
            options={FONT_OPTIONS.map(font => ({ value: font.id, label: font.name }))} />
        </label>
        <p className="text-sm text-[var(--color-ink-mute)]">Applies throughout the application. YAML and code use a fixed-width font. All fonts are available offline.</p>
        <div aria-label="Font preview" className="border p-4 grid gap-2">
          <h3 className="text-lg font-bold normal-case">Software Engineer at Acme</h3>
          <p>Explore the posting, save a note, and track your next step.</p>
          <p className="text-sm text-[var(--color-ink-mute)]">Remote · $45.25–55.75/hour · 09:00</p>
        </div>
      </section>
      <Rule />

      {/* Interface & Motion Controls */}
      <div className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
        <Toggle
          checked={settings.ui_tooltips_enabled ?? true}
          onChange={handleToggleTooltips}
          label="GLOBAL TOOLTIPS"
          description="Displays contextual hints on hover across navigation tabs, table headers, and action buttons."
        />

        <div className="border-t border-[var(--color-ink)]/20" />

        <Toggle
          checked={settings.ui_reduce_motion}
          onChange={handleToggleReduceMotion}
          label="REDUCE MOTION"
          description="Disables transitions and animations for instant rendering."
        />
      </div>

    </div>
  );
}
