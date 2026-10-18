# Deviations from spec.txt

`spec.txt` is the design contract. The implementation keeps its intent, but several signatures and
module names differ. This page records each difference and the reason, so downstream crates can
map the spec onto the code.

## Tree and node types

| Spec | Implementation | Why |
| --- | --- | --- |
| `BoxNode` with public `children` and `parent` fields | `BoxTree` owns the nodes. Read a node with `tree.get(id)`, its children with `tree.children(id)`, and its parent with `tree.parent(id)` | An arena with `NodeId` handles avoids borrowing one node while editing another, and lets ids outlive edits safely. |
| `layout_document(root: &mut BoxNode, all_nodes: &mut [BoxNode], ...)` | `layout_document(&mut BoxTree, root: NodeId, ...)` | The tree already holds every node, so passing a second slice would duplicate it. |
| `ComputedLayout` with `f32` fields | Same fields, read from `node.layout`. Layout is computed internally in 1/64 px fixed point | Fixed point makes the output identical across platforms. `f32` appears only at the output boundary. |

## Layout entry points

| Spec | Implementation | Why |
| --- | --- | --- |
| `layout_flexbox(container, children, width, height)` | Public as `layout_document`. Flex runs inside it for any container with `Display::Flex` | Each mode needs the whole tree (intrinsic sizes, deferred positioned boxes), so a per-mode public entry point would leak internals. |
| `layout_grid(...)` | Same as flexbox: runs inside `layout_document` | As above. |
| `layout_flow(...)` | Public, and takes `NodeId` and `Px` | Matches the tree and fixed-point decisions above. |
| `layout_document(root, all_nodes, constraints, ctx) -> LayoutResult` | `layout_document(tree, root, LayoutConstraints, &LayoutContext) -> Result<LayoutResult, TreeError>` | Errors are returned rather than panicking: an over-deep tree is `TreeError::TooDeep`. |

## Pagination

| Spec | Implementation | Why |
| --- | --- | --- |
| `PageLayout { pages, total_height, ... }` with `Page { width, height, content_offset_y, ... }` in `f32` | Same fields. `Page::boxes` holds `NodeId`s, and `PageLayout::fragments` gives where each box or line range lands | Pages do not move boxes. Consumers draw with fragment offsets, which keeps tree coordinates continuous. |
| Module `page` | Module `pagination` | Name matches what it does. |
| Module `box_model` | Module `tree` and `style` | Split into the arena and the style types. |

## Layout context

| Spec | Implementation | Why |
| --- | --- | --- |
| `LayoutContext` holds font metrics from `tpt-text` | `LayoutContext` holds a `TextMeasurer` and a DPI | `tpt-text` does not exist yet. The measurer trait is the boundary: real metrics plug in there. |

## Other crates in the ecosystem

- `tpt-text`, `tpt-canvas` and `tpt-math` are not available. `TextMeasurer` stands in for
  `tpt-text`, `Canvas` is a stub trait with no implementation outside the SVG debug renderer, and
  `math.rs` only holds `Edges` and `Rect`.
- `tpt-testkit` is not used. Golden and benchmark helpers are written inside the tests and examples.

## Conformance

- The spec asks for the official web-platform-tests suites. The repository adapts a sample of them
  (`tests/wpt_*.rs`). Each adapted test names its source. The rest of the suites are not
  reproduced.

## Open items

- A cross-crate `Canvas` implementation for `tpt-canvas` does not exist yet.
- The spec's `tpt-doc` milestone (PDF output with correct page breaks) depends on `tpt-doc`, which
  is outside this crate.
