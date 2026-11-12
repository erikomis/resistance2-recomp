// The Controls page's "Modern layout": the controller bindings of a modern
// shooter, laid over the game's own default controls (Options > Controls,
// "Default controls": R1 primary fire, R2 alt. fire, L1 zoom, L2 sprint /
// crouch, ○ use grenade, ✕ jump, □ reload, △ weapon select, R3 melee, L3
// nothing). PS3 control -> the physical inputs (SDL names) that press it.
// No imports, so it can be checked on its own.

export type PadBindings = Record<string, string[]>;

export const MODERN: PadBindings = {
  r1: ["righttrigger"], // R2 fires
  l1: ["lefttrigger"], // L2 zooms
  r2: ["leftshoulder"], // L1 alt. fire
  circle: ["b", "rightshoulder"], // R1 throws a grenade; ○ still does (and backs out of menus)
  l2: ["leftstick"], // L3 sprints / crouches
  l3: [], // the stick click has a new job; L3 does nothing in the game
};

/** `controller` with `layout` applied: the layout's controls get its inputs,
 * and those inputs leave every other control, so no input presses two
 * controls. Applying it twice changes nothing. */
export function applyLayout(controller: PadBindings, layout: PadBindings): PadBindings {
  const used = new Set(Object.values(layout).flat());
  const out: PadBindings = {};
  for (const [k, v] of Object.entries(controller)) out[k] = v.filter((n) => !used.has(n));
  for (const [k, v] of Object.entries(layout)) out[k] = [...v];
  return out;
}
