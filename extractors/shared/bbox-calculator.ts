// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Bounding box for an element.
 */
export interface BBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * Get the bounding box of an element relative to the viewport.
 */
export function getBBox(element: Element): BBox {
  const rect = element.getBoundingClientRect();
  return {
    x: rect.left,
    y: rect.top,
    w: rect.width,
    h: rect.height,
  };
}
