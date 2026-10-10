import { parse as parseYaml, stringify as stringifyYaml } from "yaml";
import type { SourceKind } from "../bindings/SourceKind";
import type { SourceCadence } from "../bindings/SourceCadence";

export interface YamlJobSpec {
  version?: number;
  company: string;
  url: string;
  cadence?: {
    type: SourceCadence;
    cron?: string;
  };
  engine: {
    kind: SourceKind;
    selectors?: {
      item?: string;
      title?: string;
      link?: string;
      location?: string;
      salary?: string;
    };
    regex?: {
      title?: string;
      link?: string;
      location?: string;
      salary?: string;
    };
  };
}

export function sourceToYaml(source: {
  name: string;
  identifier: string;
  kind: SourceKind;
  cadence?: SourceCadence;
  cadence_cron?: string | null;
  config?: any;
}): string {
  const cfg = source.config || {};
  const selectors = cfg.selectors || {};

  const regex: Record<string, string> = {};
  if (cfg.title_regex) regex.title = cfg.title_regex;
  if (cfg.link_regex) regex.link = cfg.link_regex;
  if (cfg.location_regex) regex.location = cfg.location_regex;
  if (cfg.salary_regex) regex.salary = cfg.salary_regex;

  const spec: YamlJobSpec = {
    version: 1,
    company: source.name,
    url: source.identifier,
    cadence: {
      type: source.cadence || "global",
      ...(source.cadence_cron ? { cron: source.cadence_cron } : {}),
    },
    engine: {
      kind: source.kind,
      ...(Object.keys(selectors).length > 0 ? { selectors } : {}),
      ...(Object.keys(regex).length > 0 ? { regex } : {}),
    },
  };
  return stringifyYaml(spec);
}

export function yamlToSource(yamlStr: string): {
  name: string;
  identifier: string;
  kind: SourceKind;
  cadence: SourceCadence;
  cadence_cron?: string | null;
  config: Record<string, any>;
} {
  const doc = parseYaml(yamlStr);
  if (!doc || typeof doc !== "object") {
    throw new Error("Invalid YAML structure: expected an object");
  }
  const company = String(doc.company || doc.name || "").trim();
  const url = String(doc.url || doc.identifier || "").trim();
  const kind = (doc.engine?.kind || doc.kind || "selector") as SourceKind;
  const rawCadence = doc.cadence?.type || (typeof doc.cadence === "string" ? doc.cadence : null);
  let cadence: SourceCadence = "global";
  if (rawCadence === "custom_cron" || rawCadence === "cron") {
    cadence = "cron";
  } else if (rawCadence === "hourly" || rawCadence === "every_1h") {
    cadence = "hourly";
  } else if (rawCadence === "every_6h") {
    cadence = "every_6h";
  } else if (rawCadence === "every_12h") {
    cadence = "every_12h";
  } else if (rawCadence === "daily") {
    cadence = "daily";
  } else if (rawCadence === "weekly") {
    cadence = "weekly";
  } else if (rawCadence === "manual") {
    cadence = "manual";
  }

  const cadence_cron = doc.cadence?.cron || doc.cadence_cron || null;

  const selectors = doc.engine?.selectors || doc.selectors || {};
  const regex = doc.engine?.regex || doc.regex || {};

  const config: Record<string, any> = {};
  if (selectors && typeof selectors === "object" && Object.keys(selectors).length > 0) {
    config.selectors = {
      ...(selectors.item ? { item: String(selectors.item) } : {}),
      ...(selectors.title ? { title: String(selectors.title) } : {}),
      ...(selectors.link ? { link: String(selectors.link) } : {}),
      ...(selectors.location ? { location: String(selectors.location) } : {}),
      ...(selectors.salary ? { salary: String(selectors.salary) } : {}),
    };
  }

  if (regex && typeof regex === "object") {
    if (regex.title) config.title_regex = String(regex.title);
    if (regex.link) config.link_regex = String(regex.link);
    if (regex.location) config.location_regex = String(regex.location);
    if (regex.salary) config.salary_regex = String(regex.salary);
  }

  return {
    name: company,
    identifier: url,
    kind,
    cadence,
    cadence_cron,
    config,
  };
}
