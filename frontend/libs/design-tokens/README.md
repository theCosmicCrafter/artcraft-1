# @storyteller/design-tokens

Shared CSS custom properties for every new ArtCraft Studio UI surface
(`medialibrary/`, `agentcopilot/`, `flows/`, `darkroom/` — see
[UI_UX_PLAN.md](../../../../artcraft-studio-plan/UI_UX_PLAN.md)).

Implements exactly the token set proposed in
[`_docs/pagedraw_design_review.md`](../../../_docs/pagedraw_design_review.md) §6, which
diagnosed this monorepo's top P0 UI-architecture gap: no shared token layer between `pagedraw`
and its hosts, so components both reach for host tokens *and* hardcode their own greys. This
package is the fix — established once, up front, before any new Phase 1 UI ships, per that
review's own recommendation and UI_UX_PLAN.md §3.

## Usage

```css
@import "@storyteller/design-tokens/tokens.css";
```

Once per host app (not per-library). Provides `--st-canvas`, `--st-accent`, `--st-hairline`,
`--st-inset-hi` (color/surface), `--st-ease`/`--st-dur-fast`/`--st-dur` (motion), and
`--st-r-shell`/`--st-r-card`/`--st-r-control` (concentric radius scale), plus a Tailwind v4
`@theme inline` mapping so `bg-st-canvas`/`rounded-st-card`-style utility classes work directly.

Composes with hosts that already have shadcn tokens (`apps/artcraft-dashboard`, `apps/genhub`)
and hosts that don't yet (`apps/artcraft`, where UI_UX_PLAN.md's Phase 1 surfaces mount first) —
see the comment block at the top of `src/tokens.css` for exactly how the fallback works.

## Status

Tokens only — no components. This package intentionally does nothing else; component libraries
(`medialibrary/` etc.) consume these tokens, they don't live here.
