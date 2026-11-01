# Workspace refinement: long forms, examples and Quit

The owner's real Rascal test exposed confusing example feedback and a form that extended the
whole page. The saved example did restore its image and panorama coordinates; its optional
projection parameter had been unset in the original run. Opening it correctly created a draft
without executing another run or loading a previous result.

The connected desktop workspace now fits the viewport, with named, keyboard-focusable Input and
Result scroll regions and persistent Run/inspection/export controls. Narrow or short viewports
retain ordinary page flow. Example loading is explicit in the result area, explains how to run,
and offers history navigation. Opening history clears the previous example's draft label.

Quit is a muted power icon with a tooltip, accessible name and 45px target. It still opens the
existing confirmation drawer. The owner considered foreground hosting, then explicitly chose
to retain H1 and requested that contract conflicts be raised before recommending changes. No
launcher lifetime, shutdown backend, CLI or shared contract was changed.

Verification: typecheck, 10 UI unit tests, reproducible embedded assets, the 60-case Chromium/
WebKit suite, and targeted reruns of six example/layout/real-Rascal cases passed. The final Quit
icon and layout rerun passed six cases, including decline, confirmation, explicit force after
graceful shutdown waits, tooltip/accessibility and target size. Desktop dimensions 1440×900 and
1024×768, narrow 320×900 and short 1280×500 were exercised; real Rascal was also inspected at
1440×900. Independent review found no remaining material issue. No paid calls were made.
