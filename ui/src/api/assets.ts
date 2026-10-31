import type { LocalApi } from "./client";

export type Asset = { id: string; sha256: string; size: number; width: number; height: number; media_type: "image/png" | "image/jpeg" };

export function isAsset(value: unknown): value is Asset {
  if (!value || typeof value !== "object") return false;
  const asset = value as Partial<Asset>;
  return typeof asset.id === "string" && /^[a-f0-9]{32}$/.test(asset.id)
    && typeof asset.sha256 === "string" && /^[a-f0-9]{64}$/.test(asset.sha256)
    && Number.isInteger(asset.size) && Number.isInteger(asset.width) && Number.isInteger(asset.height)
    && (asset.size ?? 0) > 0 && (asset.width ?? 0) > 0 && (asset.height ?? 0) > 0
    && (asset.media_type === "image/png" || asset.media_type === "image/jpeg");
}

export function assetClient(api: LocalApi) {
  return {
    async list(): Promise<Asset[]> {
      const values: unknown = await (await api.request("/api/assets")).json();
      if (!Array.isArray(values) || !values.every(isAsset)) throw new Error("The saved image list could not be read.");
      return values;
    },
    async upload(file: File): Promise<Asset> {
      const asset: unknown = await (await api.request("/api/assets", { method: "POST", headers: { "X-Wafflehouse-Upload": "1", "Content-Type": "application/octet-stream" }, body: file })).json();
      if (!isAsset(asset)) throw new Error("The imported image could not be read.");
      return asset;
    },
    async preview(asset: Asset): Promise<Blob> {
      return (await api.request(`/api/assets/${asset.id}`)).blob();
    },
  };
}

export function moveImage(images: readonly Asset[], from: number, to: number): Asset[] {
  const moved = [...images];
  if (from < 0 || to < 0 || from >= moved.length || to >= moved.length) return moved;
  const [image] = moved.splice(from, 1);
  if (image) moved.splice(to, 0, image);
  return moved;
}

export function cardinalityError(count: number, min: number, max: number): string | null {
  if (count < min) return `Choose at least ${min} image${min === 1 ? "" : "s"}.`;
  if (count > max) return `Choose no more than ${max} image${max === 1 ? "" : "s"}.`;
  return null;
}
