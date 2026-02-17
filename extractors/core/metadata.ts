// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

export interface Metadata {
  title: string;
  description: string;
  canonicalUrl: string | null;
  language: string;
  robotsDirectives: string[];
  schemaOrgTypes: string[];
  openGraph: Record<string, string>;
  jsonLd: Record<string, unknown>[];
  hasStructuredData: boolean;
  structuredDataPropertyCount: number;
}

/**
 * Extract page metadata: JSON-LD, schema.org, OpenGraph, meta tags.
 */
export function extractMetadata(doc: Document): Metadata {
  const title = doc.title ?? "";
  const description = getMetaContent(doc, "description") ?? "";
  const canonical = doc.querySelector<HTMLLinkElement>("link[rel='canonical']");
  const lang = doc.documentElement.getAttribute("lang") ?? "";

  // Robots directives
  const robotsDirectives: string[] = [];
  const robotsMeta = getMetaContent(doc, "robots");
  if (robotsMeta) {
    robotsDirectives.push(...robotsMeta.split(",").map((s) => s.trim()));
  }

  // OpenGraph
  const openGraph: Record<string, string> = {};
  doc.querySelectorAll('meta[property^="og:"]').forEach((el) => {
    const prop = el.getAttribute("property") ?? "";
    const content = el.getAttribute("content") ?? "";
    openGraph[prop.replace("og:", "")] = content;
  });

  // JSON-LD
  const jsonLd: Record<string, unknown>[] = [];
  const schemaOrgTypes: string[] = [];
  doc.querySelectorAll('script[type="application/ld+json"]').forEach((el) => {
    try {
      const data = JSON.parse(el.textContent ?? "{}") as Record<string, unknown>;
      jsonLd.push(data);
      if (typeof data["@type"] === "string") {
        schemaOrgTypes.push(data["@type"]);
      }
      if (Array.isArray(data["@type"])) {
        schemaOrgTypes.push(...(data["@type"] as string[]));
      }
    } catch {
      // Ignore invalid JSON-LD
    }
  });

  // Count schema.org microdata properties
  const microdataProps = doc.querySelectorAll("[itemprop]");
  microdataProps.forEach((el) => {
    const itemtype = el.closest("[itemtype]")?.getAttribute("itemtype") ?? "";
    if (itemtype.includes("schema.org")) {
      const typeName = itemtype.split("/").pop() ?? "";
      if (typeName && !schemaOrgTypes.includes(typeName)) {
        schemaOrgTypes.push(typeName);
      }
    }
  });

  const structuredDataPropertyCount =
    microdataProps.length +
    jsonLd.reduce((sum, ld) => sum + Object.keys(ld).length, 0);

  return {
    title,
    description,
    canonicalUrl: canonical?.href ?? null,
    language: lang,
    robotsDirectives,
    schemaOrgTypes,
    openGraph,
    jsonLd,
    hasStructuredData: schemaOrgTypes.length > 0 || jsonLd.length > 0,
    structuredDataPropertyCount,
  };
}

function getMetaContent(doc: Document, name: string): string | null {
  const el = doc.querySelector(`meta[name="${name}"]`) ??
    doc.querySelector(`meta[name="${name.toLowerCase()}"]`);
  return el?.getAttribute("content") ?? null;
}

if (typeof window !== "undefined") {
  (window as Record<string, unknown>).__cortex_extractMetadata = extractMetadata;
}
