import type { CatalogItem } from "../types/store";

export function filterCatalogItems(
  catalog: CatalogItem[],
  category: string,
  searchQuery: string,
): CatalogItem[] {
  const q = searchQuery.trim().toLowerCase();
  return catalog.filter((item) => {
    const matchesCategory = category === "all" || item.entry.category === category;
    if (!matchesCategory) return false;
    if (!q) return true;
    return (
      item.entry.engine.toLowerCase().includes(q) ||
      item.entry.version.toLowerCase().includes(q) ||
      item.entry.id.toLowerCase().includes(q) ||
      item.entry.category.toLowerCase().includes(q)
    );
  });
}

export interface EngineGroup {
  engine: string;
  category: string;
  versions: CatalogItem[];
}

export function groupCatalogByEngine(items: CatalogItem[]): EngineGroup[] {
  const groups = new Map<string, EngineGroup>();
  for (const item of items) {
    const key = item.entry.engine.toLowerCase();
    const existing = groups.get(key);
    if (existing) {
      existing.versions.push(item);
    } else {
      groups.set(key, {
        engine: item.entry.engine,
        category: item.entry.category,
        versions: [item],
      });
    }
  }
  return Array.from(groups.values());
}

export function computeDownloadPercent(downloaded: number, total: number): number {
  if (total <= 0) return 0;
  return Math.min(100, Math.round((downloaded / total) * 100));
}

export function formatTransferRate(bytesPerSec: number): string {
  if (bytesPerSec <= 0) return "";
  if (bytesPerSec >= 1024 * 1024) {
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
  }
  return `${Math.round(bytesPerSec / 1024)} KB/s`;
}
