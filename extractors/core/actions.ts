// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

import { isVisible } from "../shared/visibility-checker";
import { getBBox, BBox } from "../shared/bbox-calculator";

export interface ActionRecord {
  opcode: [number, number];
  label: string;
  targetNode: number;
  costHint: number;
  risk: number;
  visible: boolean;
  bbox: BBox;
}

/**
 * Extract all interactable actions from the document.
 */
export function extractActions(doc: Document): ActionRecord[] {
  const actions: ActionRecord[] = [];

  // Buttons
  const buttons = doc.querySelectorAll("button, [role='button'], input[type='submit'], input[type='button']");
  buttons.forEach((el) => {
    const text = (el.textContent ?? el.getAttribute("value") ?? "").trim().toLowerCase();
    const opcode = classifyButtonAction(text, el);
    actions.push({
      opcode,
      label: text || "button",
      targetNode: -1,
      costHint: 0,
      risk: classifyRisk(opcode, text),
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Links that look like actions (onclick, javascript:, etc)
  const actionLinks = doc.querySelectorAll("a[onclick], a[href^='javascript:']");
  actionLinks.forEach((el) => {
    const text = (el.textContent ?? "").trim().toLowerCase();
    actions.push({
      opcode: [0x00, 0x03], // follow_link
      label: text || "action link",
      targetNode: -2,
      costHint: 0,
      risk: 0,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Form inputs
  const inputs = doc.querySelectorAll("input[type='text'], input[type='email'], input[type='search'], input[type='password'], textarea");
  inputs.forEach((el) => {
    const name = el.getAttribute("name") ?? el.getAttribute("id") ?? "field";
    const inputType = el.getAttribute("type") ?? "text";
    actions.push({
      opcode: [0x03, 0x00], // fill_text
      label: `fill ${name} (${inputType})`,
      targetNode: -1,
      costHint: 0,
      risk: 0,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Select dropdowns
  const selects = doc.querySelectorAll("select");
  selects.forEach((el) => {
    const name = el.getAttribute("name") ?? el.getAttribute("id") ?? "select";
    actions.push({
      opcode: [0x03, 0x01], // select_option
      label: `select ${name}`,
      targetNode: -1,
      costHint: 0,
      risk: 0,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  // Checkboxes
  const checkboxes = doc.querySelectorAll("input[type='checkbox']");
  checkboxes.forEach((el) => {
    const name = el.getAttribute("name") ?? "checkbox";
    actions.push({
      opcode: [0x03, 0x02], // toggle_checkbox
      label: `toggle ${name}`,
      targetNode: -1,
      costHint: 0,
      risk: 0,
      visible: isVisible(el),
      bbox: getBBox(el),
    });
  });

  return actions;
}

function classifyButtonAction(text: string, el: Element): [number, number] {
  // Commerce
  if (text.includes("add to cart") || text.includes("add to bag")) return [0x02, 0x00];
  if (text.includes("buy now") || text.includes("purchase")) return [0x02, 0x03];
  if (text.includes("remove") && (text.includes("cart") || text.includes("bag"))) return [0x02, 0x01];
  if (text.includes("wishlist") || text.includes("save for later")) return [0x02, 0x05];

  // Auth
  if (text.includes("sign in") || text.includes("log in") || text.includes("login")) return [0x04, 0x00];
  if (text.includes("sign up") || text.includes("register")) return [0x04, 0x02];
  if (text.includes("log out") || text.includes("sign out")) return [0x04, 0x01];

  // Form
  if (text.includes("submit") || el.getAttribute("type") === "submit") return [0x03, 0x05];
  if (text.includes("next") || text.includes("continue")) return [0x03, 0x07];
  if (text.includes("back") || text.includes("previous")) return [0x03, 0x08];

  // Search
  if (text.includes("search")) return [0x01, 0x00];
  if (text.includes("filter")) return [0x01, 0x01];
  if (text.includes("sort")) return [0x01, 0x03];

  // Media
  if (text.includes("play")) return [0x05, 0x00];
  if (text.includes("download")) return [0x05, 0x04];

  // Social
  if (text.includes("like")) return [0x06, 0x00];
  if (text.includes("share")) return [0x06, 0x01];

  // System
  if (text.includes("close") || text.includes("dismiss")) return [0x07, 0x00];
  if (text.includes("accept") && text.includes("cookie")) return [0x07, 0x01];

  // Default: navigation follow_link
  return [0x00, 0x03];
}

function classifyRisk(opcode: [number, number], text: string): number {
  // Destructive
  if (text.includes("delete") || text.includes("remove")) return 2;
  if (opcode[0] === 0x02 && opcode[1] === 0x03) return 2; // buy_now

  // Cautious
  if (opcode[0] === 0x02) return 1; // commerce actions
  if (opcode[0] === 0x03 && opcode[1] === 0x05) return 1; // submit_form
  if (opcode[0] === 0x04) return 1; // auth actions

  // Safe
  return 0;
}

if (typeof window !== "undefined") {
  (window as Record<string, unknown>).__cortex_extractActions = extractActions;
}
