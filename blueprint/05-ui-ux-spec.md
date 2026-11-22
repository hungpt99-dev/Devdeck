# Deliverable 5 — UI/UX specification

## Design principles

Native, keyboard-first, local-first, and honest: every visual operation has an equivalent command/tool call, configuration files stay the source of truth, and no visual editor may silently rewrite a file in a lossy way.

## Global shell (DESIGN)

```
+------------------------------------------------------------------------------+
| Devdeck   [project / branch]                       [Ctrl+K]  [EN|VI] [Theme]  |
+----------+---------------------------------------------------+---------------+
| Activity | Tabs + editor / active workspace                   | Context side  |
| rail     |                                                   | bar (optional)|
|          |                                                   |               |
+----------+---------------------------------------------------+---------------+
| Primary  | Bottom panel: Terminal | Problems | Output | Logs |               |
| sidebar  +---------------------------------------------------+---------------+
| (per ws) | Status: branch, tool count, locale, permission state               |
+----------+---------------------------------------------------+---------------+
```

State precedence: built-in defaults < user settings < profile bundle < workspace settings < named snapshot. Snapshots restore layout/tabs/panel/env without overwriting config files. The global Ctrl/Cmd+K palette dispatches to typed tools and echoes the CLI equivalent.

## Workspaces and interaction rules

| Workspace | Primary visual operations | CLI/advanced parity | Safety |
| --- | --- | --- | --- |
| Project Explorer | tree, search, open folder, recents | `devdeck attach`, workspace CLI | Workspace-scoped permissions |
| Code Editor | tabs, splits, diff, find, LSP diagnostics | rg-style search, editor commands | Diff review before agent edits |
| Terminal | named panes, splits, scrollback | full shell | Secret redaction; approval for destructive commands |
| Git | status, hunk staging, diff, branches, worktrees | git CLI passthrough | Confirm force-push/reset |
| Docker | containers, logs, exec, compose controls | docker/podman CLI | Confirm rm/prune/down |
| Database | schema tree, query editor, virtualized grid | SQL CLI/exports | Read-only default; confirm writes |
| API | request builder, collections, response viewer | `devdeck api run`, cURL export | Script sandbox + consent |
| Tool Center | detect/install/update/uninstall, conflicts | native manager commands | Signed/checksum verified, approval |
| Configuration Studio | schema-validated raw + focused visual editors | direct file edit | File remains source of truth |
| Environment Manager | profiles, snapshots, env vars | profile CLI | Secrets as keychain references |

## Component vocabulary and tokens

All colors/spacing/radius/typography come from `crates/ui` `Tokens` (dark, light, high-contrast). Components: button, input, list, tree, table, tabs, splitter, diff view, terminal view, progress, approval dialog, problem list. Reduced-motion respected; focus always visible; no color-only signalling.

## Cross-cutting patterns

- Command palette: diacritic-insensitive search, scoped ranking, keyboard nav (already implemented in Phase 1).
- Diff review: unified/side-by-side, hunk accept/reject, before/after apply modes.
- Approval prompts: risk tier, plan/diff preview, explicit allow/deny, audit entry.
- Progress/cancellation: streamed, bounded, cancellable; retry preserves partial work.
- Errors: actionable localized messages routed to a Problems panel; never raw stack traces.

## Accessibility decision

Upstream Iced 0.14 has no shipped AccessKit tree[340]. Keyboard navigation, visible focus and localized labels are in scope; screen-reader tree support requires an upstreamed patch/maintained fork before 1.0. Slint has AccessKit but its licence conflicts with the MIT/Apache-only policy[304].

## What NOT to build

Fake visual editors that desync from files, hidden/binary config, embedded webviews, disconnected CLI commands that bypass policy, or a separate AI chat that cannot see the workspace.

Full wireframes and per-workspace flows: `research/ux-spec.md`.

## Sources

[304] https://github.com/slint-ui/slint/blob/master/LICENSE.md
[340] https://github.com/iced-rs/iced/issues/552
