// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

import { isVisible } from "../shared/visibility-checker";

export interface NavLink {
  url: string;
  text: string;
  type: "internal" | "external" | "pagination" | "anchor" | "download" | "breadcrumb";
  visible: boolean;
}

/**
 * Extract all navigation links from the document.
 */
export function extractNavigation(doc: Document): NavLink[] {
  const links: NavLink[] = [];
  const baseUrl = doc.baseURI || window.location.href;
  const baseDomain = extractDomain(baseUrl);
  const seen = new Set<string>();

  // All anchor tags
  const anchors = doc.querySelectorAll("a[href]");
  anchors.forEach((el) => {
    const href = el.getAttribute("href");
    if (!href) return;

    const resolved = resolveUrl(href, baseUrl);
    if (!resolved) return;
    if (seen.has(resolved)) return;
    seen.add(resolved);

    const text = (el.textContent ?? "").trim().slice(0, 200);
    const linkType = classifyLink(resolved, href, text, baseDomain, el);

    links.push({
      url: resolved,
      text,
      type: linkType,
      visible: isVisible(el),
    });
  });

  return links;
}

function resolveUrl(href: string, base: string): string | null {
  // Skip javascript:, mailto:, tel: etc
  if (/^(javascript:|mailto:|tel:|data:)/i.test(href)) return null;

  try {
    const url = new URL(href, base);
    // Remove hash for deduplication
    url.hash = "";
    return url.href;
  } catch {
    return null;
  }
}

function extractDomain(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

function classifyLink(
  resolved: string,
  rawHref: string,
  text: string,
  baseDomain: string,
  el: Element
): NavLink["type"] {
  const linkDomain = extractDomain(resolved);
  const lowerText = text.toLowerCase();

  // Anchor link (same page)
  if (rawHref.startsWith("#")) return "anchor";

  // Download
  if (el.hasAttribute("download")) return "download";
  if (/\.(pdf|zip|tar|gz|exe|dmg|apk|ipa)$/i.test(resolved)) return "download";

  // Breadcrumb
  const parent = el.closest("nav[aria-label*='breadcrumb'], .breadcrumb, [class*='breadcrumb']");
  if (parent) return "breadcrumb";

  // Pagination
  if (
    lowerText === "next" ||
    lowerText === "previous" ||
    lowerText === "prev" ||
    /^[0-9]+$/.test(text.trim()) ||
    el.closest("[class*='pagination'], [aria-label*='pagination'], nav[class*='pager']")
  ) {
    return "pagination";
  }

  // External
  if (linkDomain && linkDomain !== baseDomain) return "external";

  // Internal
  return "internal";
}

if (typeof window !== "undefined") {
  (window as Record<string, unknown>).__cortex_extractNavigation = extractNavigation;
}
