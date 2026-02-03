# Command Palette - Spec + UX Flow

This spec defines the UX, scoring algorithm, accessibility, and test requirements for
**Command Palette + Instant Action Search** (bd-39y4).

---

## 1) Goals
- Provide instant keyboard-driven access to all app commands/actions.
- Enable fast fuzzy search with deterministic, explainable ranking.
- Support preview panes for commands with rich context.
- Integrate seamlessly with demo showcase navigation.

## 2) Non-Goals
- Persistent command history across sessions (future enhancement).
- Custom command registration at runtime (use static registry).
- Vim-style `:` command mode (separate feature).

---

## 3) Scope & Integration

### 3.1 Integration Points
- **Demo Showcase**: Global overlay accessible from any screen.
- **Runtime**: Integrates with existing Model/Cmd architecture.
- **Theme System**: Uses semantic colors for consistent styling.

### 3.2 Command Registry
- Static registration at compile time.
- Commands grouped by category (Navigation, Settings, Actions, Help).
- Each command has: id, title, description, category, tags[], keybinding?.

---

## 4) UX States

### 4.1 Closed (Default)
- Palette is hidden.
- Keybinding listener active for trigger.

### 4.2 Open (Empty Query)
- Palette visible with input focused.
- Shows top N actions by category/recency.
- Status: "Type to search..."

### 4.3 Open (Filtering)
- Live filtering as user types.
- Results update on every keystroke.
- Debounce: 0ms (instant, no delay).

### 4.4 Preview Mode
- Tab toggles preview panel focus.
- Preview shows: command details, keybinding, description.
- Preview content is read-only.

### 4.5 Executing
- Brief flash/highlight on selected item.
- Palette closes immediately.
- Command executes via Cmd dispatch.

### 4.6 Empty Results
- Shows "No results" message.
- Hints: "Try different keywords" + query syntax help.
- Selection state: None (no item to select).

---

## 5) Controls & Keybindings

### 5.1 Open Palette
- `Ctrl+P` (primary) - Universal trigger
- `Ctrl+Shift+P` (alternative) - If Ctrl+P conflicts
- `/` (optional) - If not used by search elsewhere

### 5.2 Navigation
- `Up` / `Down` - Move selection
- `PageUp` / `PageDown` - Jump 10 items
- `Home` / `End` - First / last item
- `Tab` - Toggle preview focus (if preview enabled)

### 5.3 Actions
- `Enter` - Execute selected command
- `Esc` - Close palette (clear query first if non-empty)
- `Backspace` - Delete char / clear query

### 5.4 Query Input
- Any printable char - Append to query
- `Ctrl+A` - Select all query text
- `Ctrl+Backspace` - Delete word

---

## 6) Scoring Algorithm

### 6.1 Match Types (Priority Order)
1. **Exact match** - Query equals title (score: 1.0)
2. **Prefix match** - Title starts with query (score: 0.9)
3. **Word-start match** - Query matches start of words (score: 0.8)
4. **Contiguous substring** - Query found contiguously (score: 0.7)
5. **Fuzzy match** - Characters found in order (score: 0.3-0.6)

### 6.2 Scoring Formula
```
score = base_match_score
      + word_boundary_bonus * 0.1
      + position_bonus * 0.05  // Earlier matches score higher
      - gap_penalty * 0.02     // Gaps between matched chars
      + tag_match_bonus * 0.15 // Query also matches a tag
```

### 6.3 Tie-Breaking (Deterministic)
When scores are equal, order by:
1. Shorter title first (more specific)
2. Alphabetical by title
3. Original registration order (stable index)

### 6.4 Fuzzy Match Details
- Case-insensitive comparison.
- Match positions tracked for highlighting.
- Skip scoring if query length > title length.

### 6.5 Example Scores
| Query    | Title              | Score  | Reason                    |
|----------|-------------------|--------|---------------------------|
| "set"    | "Settings"        | 0.90   | Prefix match              |
| "set"    | "Reset View"      | 0.80   | Word-start match          |
| "set"    | "Asset Manager"   | 0.70   | Contiguous substring      |
| "stg"    | "Settings"        | 0.55   | Fuzzy (s-t-...-g)         |
| "nav"    | "Navigation"      | 0.90   | Prefix match              |
| "gd"     | "Go to Dashboard" | 0.82   | Word-start (g-d)          |

---

## 7) Visual Design

### 7.1 Layout
```
┌─ Command Palette ────────────────────────────────────────┐
│  > search query_                                         │
├──────────────────────────────────────────────────────────┤
│  ▶ Settings                           Ctrl+,             │
│    Set Theme                                             │
│    Set Font Size                                         │
├──────────────────────────────────────────────────────────┤
│  ▶ Navigation                                            │
│    Go to Dashboard                    g d                │
│    Go to File Browser                 g f                │
└──────────────────────────────────────────────────────────┘
```

### 7.2 Selection Indicator
- Solid background highlight (semantic selection color).
- Optional: `>` or `▶` prefix for selected row.
- Match highlights: bold or colored matched characters.

### 7.3 Dimensions
- Width: 60-80% of terminal width, max 80 columns.
- Height: 40-60% of terminal height, max 20 rows.
- Position: Centered horizontally, top 1/3 vertically.

### 7.4 With Preview
```
┌─ Command Palette ─────────────────┬─ Preview ────────────┐
│  > query_                         │                      │
├───────────────────────────────────┤  Title: Settings     │
│  ▶ Settings              Ctrl+,   │  Category: Config    │
│    Set Theme                      │                      │
│    Set Font Size                  │  Opens the settings  │
│    ...                            │  panel where you can │
│                                   │  customize the app.  │
└───────────────────────────────────┴──────────────────────┘
```

---

## 8) Accessibility

### 8.1 Visual Accessibility
- Selection uses both color AND visual indicator (invert/marker).
- Minimum contrast ratio: 4.5:1 (WCAG AA).
- Match highlights distinguishable without color alone.

### 8.2 Keyboard Accessibility
- All functions reachable via keyboard.
- Focus trap: Tab cycles within palette only.
- Esc always exits (predictable escape hatch).

### 8.3 Screen Reader Considerations
- Live region announcements for result count changes.
- Selection changes announced (item title + position).

### 8.4 Graceful Degradation
- Reduced colors: Use reverse video for selection.
- No Unicode: Use `>` instead of `▶`, `*` instead of `•`.
- Small terminals: Single-column mode, no preview.

---

## 9) Failure Modes (Ledger)

| Failure Mode            | Detection                        | Recovery                          |
|------------------------|----------------------------------|-----------------------------------|
| Empty registry         | len(commands) == 0               | Show "No commands registered"     |
| Query too long         | query.len() > MAX_QUERY (128)    | Truncate, show warning            |
| No matches             | results.len() == 0               | Show "No results" + hints         |
| Command execution fail | Cmd returns Error variant        | Show error toast, keep palette    |
| Render budget exceeded | Frame budget < threshold         | Skip preview, reduce row count    |

---

## 10) Data Model

### 10.1 Command Registration
```rust
pub struct CommandEntry {
    pub id: &'static str,           // Unique identifier
    pub title: &'static str,        // Display title
    pub description: &'static str,  // Longer description
    pub category: Category,         // Grouping category
    pub tags: &'static [&'static str], // Search tags
    pub keybinding: Option<&'static str>, // Display keybinding
    pub action: fn() -> Cmd<Msg>,   // Execution function
}

#[derive(Clone, Copy)]
pub enum Category {
    Navigation,
    Settings,
    Actions,
    Help,
    Debug,
}
```

### 10.2 Search Result
```rust
pub struct SearchResult {
    pub entry: &CommandEntry,
    pub score: f64,
    pub match_positions: Vec<usize>, // For highlighting
}
```

### 10.3 Palette State
```rust
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected: Option<usize>,
    pub preview_focused: bool,
}
```

---

## 11) Performance Requirements

### 11.1 Latency Budgets
- Query → Results: < 5ms for 1000 commands
- Keystroke → Display: < 16ms (60fps target)
- Command execution: No added latency

### 11.2 Memory
- Pre-computed lowercase titles for fast matching.
- Reuse result Vec (clear + push, don't reallocate).
- Cache scoring intermediate values.

### 11.3 Optimization Opportunities
- Early exit on exact match.
- Skip scoring if query doesn't share first char with title.
- Parallel scoring for large registries (future).

---

## 12) Test Matrix

### 12.1 Unit Tests

| Test Name                          | Description                                    |
|-----------------------------------|------------------------------------------------|
| test_exact_match_highest_score    | Exact match scores 1.0                         |
| test_prefix_match_score           | "set" → "Settings" scores 0.9                  |
| test_word_start_match             | "gd" → "Go Dashboard" scores 0.82              |
| test_fuzzy_match_basic            | "stg" → "Settings" finds and scores            |
| test_no_match_returns_empty       | "xyz" returns empty results                    |
| test_score_deterministic          | Same input → identical scores                  |
| test_tiebreak_shorter_first       | Equal scores → shorter title wins              |
| test_tiebreak_alphabetical        | Equal length → alphabetical order              |
| test_tiebreak_stable_index        | Equal everything → original order              |
| test_case_insensitive             | "SET" matches "settings"                       |
| test_match_positions_correct      | Highlight positions match actual chars         |
| test_empty_query_returns_all      | Empty query returns all commands               |
| test_query_longer_than_title      | Long query → no match (quick rejection)        |
| test_special_chars_handled        | Queries with spaces, punctuation work          |

### 12.2 Snapshot Tests

| Snapshot Name                      | Description                                    |
|-----------------------------------|------------------------------------------------|
| palette_closed_80x24              | Palette hidden, normal UI visible              |
| palette_open_empty_80x24          | Empty query, showing top commands              |
| palette_open_results_80x24        | Query with 5 matching results                  |
| palette_no_results_80x24          | Query with no matches, hint shown              |
| palette_selection_highlight_80x24 | Selected item has visible highlight            |
| palette_with_preview_120x40       | Preview pane visible with command details      |
| palette_match_highlight_80x24     | Matched characters highlighted in results      |
| palette_small_terminal_40x10      | Graceful degradation in tiny terminal          |

### 12.3 E2E / PTY Tests

| Test Name                          | Actions                                        |
|-----------------------------------|------------------------------------------------|
| e2e_open_close                    | Ctrl+P → visible, Esc → hidden                 |
| e2e_type_filter                   | Type "nav" → results filter live               |
| e2e_navigate_select               | Down×3, Enter → correct command executes       |
| e2e_empty_then_type               | Backspace×N clears, new query works            |
| e2e_execute_command               | Select "Go Dashboard" → screen changes         |
| e2e_escape_clears_first           | Type "abc", Esc clears query, Esc again closes |
| e2e_preview_toggle                | Tab toggles preview focus                      |

### 12.4 Property Tests

| Property                           | Invariant                                      |
|-----------------------------------|------------------------------------------------|
| score_bounded                     | 0.0 ≤ score ≤ 1.0 for all inputs               |
| ordering_total                    | Results form a total order (no ambiguity)      |
| idempotent_search                 | search(q) == search(q) always                  |
| monotonic_prefix                  | score("ab") ≥ score("a") for any title         |
| empty_query_all                   | search("") includes all commands               |

---

## 13) Files to Create/Modify

### 13.1 New Files
- `crates/ftui-widgets/src/command_palette/mod.rs` - Widget module
- `crates/ftui-widgets/src/command_palette/widget.rs` - Palette widget
- `crates/ftui-widgets/src/command_palette/scorer.rs` - Fuzzy matcher
- `crates/ftui-widgets/src/command_palette/registry.rs` - Command registry
- `crates/ftui-demo-showcase/src/screens/command_palette.rs` - Demo screen

### 13.2 Modifications
- `crates/ftui-widgets/src/lib.rs` - Export command_palette module
- `crates/ftui-demo-showcase/src/app.rs` - Integrate global palette
- `crates/ftui-demo-showcase/src/screens/mod.rs` - Add screen

---

## 14) Implementation Phases

### Phase 1: Core (bd-39y4.2)
- Basic widget with TextInput + List.
- Simple substring matching (no fuzzy).
- Open/close with Ctrl+P.

### Phase 2: Scoring (bd-39y4.11)
- Bayesian match scoring.
- Evidence ledger for decisions.
- Match position tracking.

### Phase 3: Polish (bd-39y4.12-13)
- Conformal rank confidence.
- Performance profiling.
- Incremental scoring optimization.

### Phase 4: Integration (bd-39y4.3)
- Demo showcase integration.
- Full command registry.
- E2E tests.

---

## 15) Open Questions (Resolved)

1. **Should `/` trigger palette?**
   - Decision: Optional, configurable. Default: No (conflicts with search).

2. **Preview pane default?**
   - Decision: Hidden by default. Tab to toggle.

3. **Max visible results?**
   - Decision: 10 items default, scroll for more.

4. **Query persistence on close?**
   - Decision: Clear on close. Reopening starts fresh.

---

## 16) Docs + Demo Script (bd-39y4.6)

### 16.1 Quickstart (Demo Showcase)
- Run the demo: `cargo run -p ftui-demo-showcase`
- Open the palette: `Ctrl+K` (demo binding; `Ctrl+P` is reserved for the performance sparkline)
- Close: `Esc`
- Execute: `Enter`
- Navigate: `Up/Down`, `PageUp/PageDown`, `Home/End`

### 16.2 Action Registration (Demo)
- Global action registry lives in `crates/ftui-demo-showcase/src/app.rs`.
- Widget implementation lives in `crates/ftui-widgets/src/command_palette/mod.rs`.
- The demo uses the global registry to power the overlay (open from any screen).

### 16.3 Demo Script (E2E + Logs)
Use the dedicated script for repeatable PTY + JSONL logs:

```bash
./scripts/command_palette_e2e.sh
./scripts/command_palette_e2e.sh --quick
LOG_DIR=/tmp/ftui_palette_e2e ./scripts/command_palette_e2e.sh --verbose
```

Outputs:
- `e2e.jsonl` with step timings + metadata
- `e2e_stderr.jsonl` with test stderr (when running integration tests)

---

## Changelog

| Version | Date       | Author      | Changes                                |
|---------|------------|-------------|----------------------------------------|
| 1.0     | 2026-02-03 | BlueAnchor  | Initial spec from bd-39y4.1 bead       |
