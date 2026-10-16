import { api } from "./api";
// Share one local IPC read across every card using this content-addressed asset.
const assets = new Map<string, Promise<string>>();
export function loadCompanyLogoAsset(reference: string): Promise<string> {
  let value = assets.get(reference);
  if (!value) {
    value = api.getCompanyLogoAsset(reference).catch(error => { assets.delete(reference); throw error; });
    assets.set(reference, value);
    if (assets.size > 64) assets.delete(assets.keys().next().value!);
  }
  return value;
}

export function clearCompanyLogoCache() { assets.clear(); }
