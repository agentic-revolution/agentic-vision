// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Options for DOM traversal.
 */
export interface WalkOptions {
  includeShadowDom?: boolean;
  includeHidden?: boolean;
  maxDepth?: number;
}

/**
 * Visitor function called for each element.
 * Return false to skip the subtree.
 */
export type Visitor = (element: Element, depth: number) => boolean;

/**
 * Walk the DOM tree recursively, calling visitor for each element.
 */
export function walkDom(
  root: Element,
  visitor: Visitor,
  options: WalkOptions = {}
): void {
  const maxDepth = options.maxDepth ?? 100;
  walkNode(root, visitor, options, 0, maxDepth);
}

function walkNode(
  node: Element,
  visitor: Visitor,
  options: WalkOptions,
  depth: number,
  maxDepth: number
): void {
  if (depth > maxDepth) return;

  const shouldContinue = visitor(node, depth);
  if (!shouldContinue) return;

  // Walk children
  const children = node.children;
  for (let i = 0; i < children.length; i++) {
    walkNode(children[i], visitor, options, depth + 1, maxDepth);
  }

  // Walk shadow DOM if requested
  if (options.includeShadowDom && node.shadowRoot) {
    const shadowChildren = node.shadowRoot.children;
    for (let i = 0; i < shadowChildren.length; i++) {
      walkNode(shadowChildren[i], visitor, options, depth + 1, maxDepth);
    }
  }
}
