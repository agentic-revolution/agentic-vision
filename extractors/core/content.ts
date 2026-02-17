// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

import { isVisible } from "../shared/visibility-checker";
import { getBBox, BBox } from "../shared/bbox-calculator";

export interface ContentBlock {
  id: string;
  type: "heading" | "paragraph" | "price" | "rating" | "table" | "image" | "list" | "code" | "form";
  text: string;
  level?: number;
  value?: number;
  confidence: number;
  visible: boolean;
  bbox: BBox;
}

const PRICE_REGEX = /\$[\d,]+\.?\d{0,2}/g;
const RATING_REGEX = /(\d+\.?\d*)\s*(?:\/\s*(\d+)|out of\s*(\d+)|stars?)/i;

let blockCounter = 0;

function nextId(): string {
  blockCounter++;
  return `cb_${String(blockCounter).padStart(3, "0")}`;
}

/**
 * Extract all content blocks from the document.
 */
export function extractContent(doc: Document): ContentBlock[] {
  blockCounter = 0;
  const blocks: ContentBlock[] = [];

  // Headings
  const headings = doc.querySelectorAll("h1, h2, h3, h4, h5, h6");
  headings.forEach((el) => {
    const text = (el.textContent ?? "").trim();
    if (!text) return;
    const level = parseInt(el.tagName[1], 10);
    blocks.push({
      id: nextId(),
      type: "heading",
      text,
      level,
      confidence: 0.95,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Paragraphs
  const paragraphs = doc.querySelectorAll("p");
  paragraphs.forEach((el) => {
    const text = (el.textContent ?? "").trim();
    if (!text || text.length < 10) return;
    blocks.push({
      id: nextId(),
      type: "paragraph",
      text: text.slice(0, 500),
      confidence: 0.85,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Prices — from schema.org first, then regex
  const priceEls = doc.querySelectorAll('[itemprop="price"], [data-price], .price, .product-price');
  priceEls.forEach((el) => {
    const text = (el.textContent ?? "").trim();
    const match = text.match(PRICE_REGEX);
    if (match) {
      const value = parseFloat(match[0].replace(/[$,]/g, ""));
      blocks.push({
        id: nextId(),
        type: "price",
        text: match[0],
        value,
        confidence: el.hasAttribute("itemprop") ? 0.99 : 0.8,
        visible: isVisible(el),
        bbox: getBBox(el),
      });
    }
  });

  // Ratings — from schema.org or aria
  const ratingEls = doc.querySelectorAll('[itemprop="ratingValue"], [aria-label*="rating"], .rating, .stars');
  ratingEls.forEach((el) => {
    const text = (el.textContent ?? "").trim();
    const ariaLabel = el.getAttribute("aria-label") ?? "";
    const matchText = (text + " " + ariaLabel).match(RATING_REGEX);
    if (matchText) {
      const rating = parseFloat(matchText[1]);
      const max = parseFloat(matchText[2] || matchText[3] || "5");
      blocks.push({
        id: nextId(),
        type: "rating",
        text: `${rating}/${max}`,
        value: rating / max,
        confidence: el.hasAttribute("itemprop") ? 0.99 : 0.8,
        visible: isVisible(el),
        bbox: getBBox(el),
      });
    }
  });

  // Tables
  const tables = doc.querySelectorAll("table");
  tables.forEach((el) => {
    const rows = el.querySelectorAll("tr").length;
    blocks.push({
      id: nextId(),
      type: "table",
      text: `Table with ${rows} rows`,
      confidence: 0.9,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Images
  const images = doc.querySelectorAll("img[src]");
  images.forEach((el) => {
    const alt = el.getAttribute("alt") ?? "";
    const src = el.getAttribute("src") ?? "";
    blocks.push({
      id: nextId(),
      type: "image",
      text: alt || src.split("/").pop() || "image",
      confidence: 0.9,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Lists
  const lists = doc.querySelectorAll("ul, ol");
  lists.forEach((el) => {
    const items = el.querySelectorAll("li").length;
    if (items === 0) return;
    blocks.push({
      id: nextId(),
      type: "list",
      text: `List with ${items} items`,
      confidence: 0.85,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  return blocks;
}

// Export for browser context
if (typeof window !== "undefined") {
  (window as Record<string, unknown>).__cortex_extractContent = extractContent;
}
