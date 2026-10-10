# Graph Report - bd-neon  (2026-10-02)

## Corpus Check
- 31 files · ~9,018 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 270 nodes · 678 edges · 10 communities
- Extraction: 59% EXTRACTED · 41% INFERRED · 0% AMBIGUOUS · INFERRED: 276 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]

## God Nodes (most connected - your core abstractions)
1. `dim_text()` - 38 edges
2. `Store` - 22 edges
3. `foreground()` - 22 edges
4. `button()` - 20 edges
5. `board()` - 18 edges
6. `issue_row()` - 18 edges
7. `card_frame()` - 18 edges
8. `neon_cyan()` - 17 edges
9. `popup_body()` - 16 edges
10. `apply()` - 16 edges

## Surprising Connections (you probably didn't know these)
- `recent_browser()` --calls--> `beads_neighbors()`  [INFERRED]
  ui/repo_switcher.rs → discover.rs
- `suggest()` --calls--> `recent_from_path()`  [INFERRED]
  discover.rs → model.rs
- `recent_browser()` --calls--> `suggest()`  [INFERRED]
  ui/repo_switcher.rs → discover.rs
- `switcher_popup()` --calls--> `looks_like_path()`  [INFERRED]
  ui/repo_switcher.rs → discover.rs
- `issue_id()` --calls--> `split_issue_id()`  [INFERRED]
  ui/badge.rs → model.rs

## Communities (10 total, 0 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.11
Nodes (39): button(), ButtonProps, ButtonProps<'a>, ButtonVariant, kanban_card(), KanbanCardProps, collapsed_lane(), expanded_lane() (+31 more)

### Community 1 - "Community 1"
Cohesion: 0.13
Nodes (34): badge(), badge_color(), issue_id(), kind_badge(), kind_color(), priority_badge(), status_badge(), time_badge() (+26 more)

### Community 2 - "Community 2"
Cohesion: 0.12
Nodes (14): matches_filter(), matches_search(), recent_from_path(), lanes_cover_all_filters(), load_state(), open_detail_seeds_preview(), open_detail_uses_list_markdown(), open_tabs_add_keep_order_and_close() (+6 more)

### Community 3 - "Community 3"
Cohesion: 0.07
Nodes (25): Action, DepFields, DepJson, DepNode, Filter, format_clock(), format_local(), format_local_day_month_year() (+17 more)

### Community 4 - "Community 4"
Cohesion: 0.09
Nodes (24): board(), header(), list_body(), loading_body(), chip(), ChipProps, footer(), FooterProps (+16 more)

### Community 5 - "Community 5"
Cohesion: 0.17
Nodes (23): beads_neighbors(), common_prefix(), complete_path(), dir_query(), looks_like_path(), matches_query(), path_complete_arbeit(), path_completions() (+15 more)

### Community 6 - "Community 6"
Cohesion: 0.14
Nodes (17): BdError, docstool_snapshot(), fabrik_snapshot(), fetch_detail(), fetch_snapshot(), list_all(), parse_json_array(), parse_show_json() (+9 more)

### Community 7 - "Community 7"
Cohesion: 0.21
Nodes (5): BdNeonApp, main(), apply_window_effect(), PersistedState, BoardView

### Community 8 - "Community 8"
Cohesion: 0.32
Nodes (6): icon_button(), IconButtonProps, title_bar_fill(), shell(), title_bar(), window_controls()

## Knowledge Gaps
- **27 isolated node(s):** `Screen`, `ViewMode`, `Issue`, `DepNode`, `IssueDetail` (+22 more)
  These have ≤1 connection - possible missing edges or undocumented components.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `dim_text()` connect `Community 1` to `Community 0`, `Community 8`, `Community 4`, `Community 5`?**
  _High betweenness centrality (0.165) - this node is a cross-community bridge._
- **Why does `issue_id()` connect `Community 1` to `Community 0`, `Community 3`?**
  _High betweenness centrality (0.139) - this node is a cross-community bridge._
- **Why does `split_issue_id()` connect `Community 3` to `Community 1`?**
  _High betweenness centrality (0.135) - this node is a cross-community bridge._
- **Are the 37 inferred relationships involving `dim_text()` (e.g. with `kind_color()` and `status_badge()`) actually correct?**
  _`dim_text()` has 37 INFERRED edges - model-reasoned connections that need verification._
- **Are the 20 inferred relationships involving `foreground()` (e.g. with `badge()` and `kind_color()`) actually correct?**
  _`foreground()` has 20 INFERRED edges - model-reasoned connections that need verification._
- **Are the 17 inferred relationships involving `button()` (e.g. with `surface_active()` and `neon_cyan()`) actually correct?**
  _`button()` has 17 INFERRED edges - model-reasoned connections that need verification._
- **Are the 14 inferred relationships involving `board()` (e.g. with `.update()` and `footer()`) actually correct?**
  _`board()` has 14 INFERRED edges - model-reasoned connections that need verification._