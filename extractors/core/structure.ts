// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

export interface PageStructure {
  hasHeader: boolean;
  hasNav: boolean;
  hasMain: boolean;
  hasSidebar: boolean;
  hasFooter: boolean;
  headingCount: number;
  paragraphCount: number;
  imageCount: number;
  videoCount: number;
  tableCount: number;
  listCount: number;
  formCount: number;
  formFieldCount: number;
  linkCountInternal: number;
  linkCountExternal: number;
  textLength: number;
  totalElements: number;
  textDensity: number;
  maxDomDepth: number;
}

/**
 * Extract structural information about the page.
 */
export function extractStructure(doc: Document): PageStructure {
  const body = doc.body;
  if (!body) {
    return emptyStructure();
  }

  const baseDomain = extractDomain(doc.baseURI || "");
  let linkCountInternal = 0;
  let linkCountExternal = 0;

  const anchors = doc.querySelectorAll("a[href]");
  anchors.forEach((el) => {
    const href = el.getAttribute("href") ?? "";
    if (href.startsWith("#") || href.startsWith("javascript:")) return;
    try {
      const url = new URL(href, doc.baseURI);
      if (url.hostname === baseDomain) {
        linkCountInternal++;
      } else {
        linkCountExternal++;
      }
    } catch {
      linkCountInternal++;
    }
  });

  const textLength = (body.textContent ?? "").length;
  const totalElements = body.querySelectorAll("*").length;
  const formFieldCount = doc.querySelectorAll(
    "input, select, textarea, button[type='submit']"
  ).length;

  // Compute max DOM depth
  let maxDepth = 0;
  function measureDepth(el: Element, depth: number): void {
    if (depth > maxDepth) maxDepth = depth;
    if (depth > 50) return; // safety limit
    const children = el.children;
    for (let i = 0; i < children.length; i++) {
      measureDepth(children[i], depth + 1);
    }
  }
  measureDepth(body, 0);

  return {
    hasHeader: !!doc.querySelector("header, [role='banner']"),
    hasNav: !!doc.querySelector("nav, [role='navigation']"),
    hasMain: !!doc.querySelector("main, [role='main']"),
    hasSidebar: !!doc.querySelector("aside, [role='complementary'], .sidebar"),
    hasFooter: !!doc.querySelector("footer, [role='contentinfo']"),
    headingCount: doc.querySelectorAll("h1, h2, h3, h4, h5, h6").length,
    paragraphCount: doc.querySelectorAll("p").length,
    imageCount: doc.querySelectorAll("img").length,
    videoCount: doc.querySelectorAll("video, iframe[src*='youtube'], iframe[src*='vimeo']").length,
    tableCount: doc.querySelectorAll("table").length,
    listCount: doc.querySelectorAll("ul, ol").length,
    formCount: doc.querySelectorAll("form").length,
    formFieldCount,
    linkCountInternal,
    linkCountExternal,
    textLength,
    totalElements,
    textDensity: totalElements > 0 ? textLength / totalElements : 0,
    maxDomDepth: maxDepth,
  };
}

function emptyStructure(): PageStructure {
  return {
    hasHeader: false, hasNav: false, hasMain: false, hasSidebar: false, hasFooter: false,
    headingCount: 0, paragraphCount: 0, imageCount: 0, videoCount: 0, tableCount: 0,
    listCount: 0, formCount: 0, formFieldCount: 0, linkCountInternal: 0, linkCountExternal: 0,
    textLength: 0, totalElements: 0, textDensity: 0, maxDomDepth: 0,
  };
}

function extractDomain(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

if (typeof window !== "undefined") {
  (window as Record<string, unknown>).__cortex_extractStructure = extractStructure;
}
