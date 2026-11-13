import { describe, expect, test } from "bun:test";
import {
  computeDownloadPercent,
  filterCatalogItems,
  formatTransferRate,
} from "../lib/storeHelpers";
import type { CatalogItem } from "../types/store";

const sampleCatalog: CatalogItem[] = [
  {
    entry: {
      id: "nginx@1.26.2",
      engine: "nginx",
      category: "web_server",
      version: "1.26.2",
      platform: "linux-x64",
      downloadUrl: "https://example.test/nginx.tar.gz",
      sha256: "abc",
      archiveFormat: "tar_gz",
      stripComponents: 1,
      stripProfile: "default",
      blueprintId: "nginx",
      estimatedSizeMb: 4,
    },
    installed: true,
    installPath: "/tmp/nginx",
  },
  {
    entry: {
      id: "php@8.3.12",
      engine: "php",
      category: "runtime",
      version: "8.3.12",
      platform: "linux-x64",
      downloadUrl: "https://example.test/php.tar.gz",
      sha256: "def",
      archiveFormat: "tar_gz",
      stripComponents: 1,
      stripProfile: "default",
      blueprintId: "php_cgi",
      estimatedSizeMb: 18,
    },
    installed: false,
    installPath: null,
  },
  {
    entry: {
      id: "mariadb@11.4.3",
      engine: "mariadb",
      category: "database",
      version: "11.4.3",
      platform: "linux-x64",
      downloadUrl: "https://example.test/mariadb.tar.gz",
      sha256: "ghi",
      archiveFormat: "tar_gz",
      stripComponents: 1,
      stripProfile: "default",
      blueprintId: "mariadb",
      estimatedSizeMb: 42,
    },
    installed: false,
    installPath: null,
  },
];

describe("storeHelpers", () => {
  test("filterCatalogItems filters by category and search query", () => {
    expect(filterCatalogItems(sampleCatalog, "all", "")).toHaveLength(3);
    expect(filterCatalogItems(sampleCatalog, "runtime", "")).toHaveLength(1);
    expect(filterCatalogItems(sampleCatalog, "all", "11.4")).toHaveLength(1);
    expect(filterCatalogItems(sampleCatalog, "web_server", "nginx")).toHaveLength(1);
    expect(filterCatalogItems(sampleCatalog, "database", "nginx")).toHaveLength(0);
  });

  test("computeDownloadPercent clamps between 0 and 100", () => {
    expect(computeDownloadPercent(0, 0)).toBe(0);
    expect(computeDownloadPercent(50, 200)).toBe(25);
    expect(computeDownloadPercent(250, 200)).toBe(100);
  });

  test("formatTransferRate formats KB/s and MB/s", () => {
    expect(formatTransferRate(0)).toBe("");
    expect(formatTransferRate(512 * 1024)).toBe("512 KB/s");
    expect(formatTransferRate(2.5 * 1024 * 1024)).toBe("2.5 MB/s");
  });
});
