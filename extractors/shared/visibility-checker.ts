// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Check if an element is visible to the user.
 */
export function isVisible(element: Element): boolean {
  if (!(element instanceof HTMLElement)) return false;

  const style = window.getComputedStyle(element);

  // display: none
  if (style.display === "none") return false;

  // visibility: hidden or collapse
  if (style.visibility === "hidden" || style.visibility === "collapse") return false;

  // opacity: 0
  if (parseFloat(style.opacity) === 0) return false;

  // Zero dimensions
  const rect = element.getBoundingClientRect();
  if (rect.width === 0 && rect.height === 0) return false;

  // Overflow clipping — element entirely outside parent
  if (style.overflow === "hidden") {
    if (rect.width <= 1 && rect.height <= 1) return false;
  }

  return true;
}
