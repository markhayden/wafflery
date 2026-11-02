import { fileURLToPath } from 'node:url';

// Native scrollbar appearance/gutters depend on the macOS preference, not our UI.
// Normalize only during capture; interaction/overflow checks keep native scrolling.
export const visualPolicy = {
  animations: 'disabled' as const,
  stylePath: fileURLToPath(new URL('./visual-scrollbars.css', import.meta.url)),
  threshold: 0.2,
  maxDiffPixelRatio: 0.01,
};
