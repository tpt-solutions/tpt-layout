//! Style inputs for each box. CSS parsing and cascade are out of scope: the application fills
//! these values in directly. Everything here is `Copy` so layout can read styles while it
//! mutates the tree.

use crate::{math::Edges, px::Px};

/// A length-like style value.
/// Writing mode of a box's content. A box whose mode is `Inherit` (the default) takes its parent's
/// effective mode, as `writing-mode` is inherited in CSS. Any other value starts a new writing
/// mode: a box whose effective mode differs from its parent's is a *root* of that mode, and its
/// subtree is laid out in the root's own logical space and mapped back.
///
/// Sizes, margins, paddings, borders and insets are authored in physical (page) terms. Inside a
/// root they are converted to the root's logical space, laid out there, and mapped back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WritingMode {
    /// Takes the parent's effective mode. At the top of a tree the effective mode is
    /// `HorizontalTb`.
    #[default]
    Inherit,
    /// Lines run left to right and blocks stack top to bottom.
    HorizontalTb,
    /// Lines run top to bottom and blocks stack right to left.
    VerticalRl,
    /// Lines run top to bottom and blocks stack left to right.
    VerticalLr,
    /// Like `VerticalRl`, with sideways glyphs. Lines run top to bottom.
    SidewaysRl,
    /// Lines run bottom to top and blocks stack left to right.
    SidewaysLr,
}

impl WritingMode {
    pub fn is_vertical(self) -> bool {
        matches!(
            self,
            WritingMode::VerticalRl
                | WritingMode::VerticalLr
                | WritingMode::SidewaysRl
                | WritingMode::SidewaysLr
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dimension {
    #[default]
    Auto,
    Length(Px),
    /// Hundredths of a percent (`5000` = 50%), resolved against a base length.
    Percent(i32),
}

impl Dimension {
    pub const ZERO: Dimension = Dimension::Length(Px::ZERO);

    pub const fn px(px: i32) -> Self {
        Dimension::Length(Px::from_px(px))
    }

    /// Builds a percentage from a float (`50.0` = 50%).
    pub fn percent(percent: f32) -> Self {
        Dimension::Percent(crate::px::round_half_away(percent * 100.0))
    }

    /// Resolves against `base`. `Auto` resolves to `None`, and so does a percentage whose base is
    /// indefinite (for example a percentage height inside an auto-height parent).
    pub fn resolve(self, base: Option<Px>) -> Option<Px> {
        match self {
            Dimension::Auto => None,
            Dimension::Length(v) => Some(v),
            Dimension::Percent(h) => base.map(|b| b.percent(h)),
        }
    }
}

/// Outer display type.
///
/// Text leaves are block-level unless their display is `Inline`. `Inline` and `InlineBlock`
/// children of a block container form inline formatting contexts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
    #[default]
    Block,
    Inline,
    InlineBlock,
    Flex,
    Grid,
    Table,
    TableRow,
    TableCell,
}

impl Display {
    /// Participates in an inline formatting context rather than block flow.
    pub fn is_inline_level(self) -> bool {
        matches!(self, Display::Inline | Display::InlineBlock)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Float {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Clear {
    #[default]
    None,
    Left,
    Right,
    Both,
}

/// Forced and avoided breaks for pagination and columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Break {
    #[default]
    Auto,
    Avoid,
    Always,
    /// Break to the next page on the left (odd-numbered pages in the page sequence are right).
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Cross-axis (or, for grid, justify/align within a cell) alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignContent {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
}

/// Maximum explicit tracks per axis. Fixed capacity keeps `BoxStyle` `Copy`.
pub const MAX_TRACKS: usize = 16;
/// Maximum grid-template-areas dimension. Area ids are `1..=255`; `0` means no area.
pub const MAX_AREA_DIM: usize = 12;

/// A grid track size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackSize {
    /// Fixed px or percentage of the grid container.
    Fixed(Dimension),
    /// Flexible share of leftover space, in hundredths (`100` = `1fr`).
    Fr(i32),
    /// Sized to the largest item in the track (min-content and max-content are treated alike).
    #[default]
    Auto,
    /// `minmax(min, max)` with px or percentage bounds. `Auto` means content-sized.
    MinMax { min: Dimension, max: Dimension },
}

/// Explicit track list, in order. Tracks beyond `len` are implicit (`grid_auto_*`).
///
/// `line_names[i]` names grid line `i` (0-based: line 0 is before the first track). `0` means
/// unnamed. Lines are named with [`Tracks::with_line_name`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tracks {
    pub items: [TrackSize; MAX_TRACKS],
    pub len: u8,
    pub line_names: [u8; MAX_TRACKS + 1],
}

impl Tracks {
    /// Panics if more than `MAX_TRACKS` tracks are given.
    pub fn new(sizes: &[TrackSize]) -> Self {
        assert!(sizes.len() <= MAX_TRACKS, "too many explicit tracks");
        let mut items = [TrackSize::Auto; MAX_TRACKS];
        items[..sizes.len()].copy_from_slice(sizes);
        Tracks {
            items,
            len: sizes.len() as u8,
            line_names: [0; MAX_TRACKS + 1],
        }
    }

    /// Names the grid line before track `line` (0-based; `len` names the line after the last
    /// track). `name` is `1..=255`; `0` clears the name. Lines beyond the track list are ignored.
    pub fn with_line_name(mut self, line: usize, name: u8) -> Self {
        if line <= self.len as usize {
            self.line_names[line] = name;
        }
        self
    }

    pub fn as_slice(&self) -> &[TrackSize] {
        &self.items[..self.len as usize]
    }
}

/// Grid container template: explicit tracks, line names and named areas. Kept on the node, not in
/// `BoxStyle`, so that styles stay small.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridTemplate {
    pub columns: Tracks,
    pub rows: Tracks,
    /// Row-major named areas. `0` is an empty cell.
    pub areas: [[u8; MAX_AREA_DIM]; MAX_AREA_DIM],
}

/// A grid line reference for one side of an item's placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridLine {
    #[default]
    Auto,
    /// 1-based line index. Negative values count from the end (`-1` is the last line).
    Index(i32),
    /// Span of this many tracks (only meaningful for the end side, or alone for the start side).
    Span(u32),
    /// A named line from the track list (`Tracks::with_line_name`). `0` matches nothing.
    Named(u8),
}

/// Placement of a grid item on one axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridPlacement {
    pub start: GridLine,
    pub end: GridLine,
}

/// Style for one box. `max_*` set to `Auto` means no maximum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxStyle {
    pub display: Display,
    pub direction: Direction,
    pub writing_mode: WritingMode,
    pub position: Position,
    pub box_sizing: BoxSizing,
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,
    pub min_height: Dimension,
    pub max_height: Dimension,
    /// `Auto` is only meaningful for horizontal margins.
    pub margin: Edges<Dimension>,
    pub padding: Edges<Dimension>,
    pub border: Edges<Px>,
    /// Offsets for `Relative` and `Absolute` boxes (top, right, bottom, left).
    pub inset: Edges<Dimension>,
    pub overflow: Overflow,
    /// Paint order within the containing stacking order. See `paint::paint_order`.
    pub z_index: Option<i32>,
    pub float: Float,
    pub clear: Clear,

    // Pagination and columns.
    pub break_before: Break,
    pub break_after: Break,
    pub break_inside: Break,
    /// Minimum lines at the bottom of a page before a break (text leaves).
    pub orphans: u16,
    /// Minimum lines at the top of a page after a break (text leaves).
    pub widows: u16,
    /// Number of columns; `0` or `1` means a single column.
    pub column_count: u16,
    /// Marks a table row to repeat at the top of each page it spans.
    pub table_header: bool,

    // Flex container.
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub row_gap: Dimension,
    pub column_gap: Dimension,

    // Flex item.
    /// `None` means `auto` (the stretch-to-fit default is `align_items`).
    pub align_self: Option<AlignItems>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub order: i32,

    // Grid container templates live on the node (see `GridTemplate`) to keep styles small.
    pub grid_auto_columns: TrackSize,
    pub grid_auto_rows: TrackSize,
    pub grid_auto_flow: GridAutoFlow,
    pub grid_auto_flow_dense: bool,
    pub justify_items: AlignItems,

    // Grid item.
    pub grid_column: GridPlacement,
    pub grid_row: GridPlacement,
    /// Named area id (from the parent grid's template areas). `0` for none.
    pub grid_area: u8,
    pub justify_self: Option<AlignItems>,
}

impl Default for BoxStyle {
    fn default() -> Self {
        BoxStyle {
            display: Display::Block,
            direction: Direction::Ltr,
            writing_mode: WritingMode::Inherit,
            position: Position::Static,
            box_sizing: BoxSizing::ContentBox,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_height: Dimension::Auto,
            margin: Edges::all(Dimension::ZERO),
            padding: Edges::all(Dimension::ZERO),
            border: Edges::all(Px::ZERO),
            inset: Edges::all(Dimension::Auto),
            overflow: Overflow::Visible,
            z_index: None,
            float: Float::None,
            clear: Clear::None,
            break_before: Break::Auto,
            break_after: Break::Auto,
            break_inside: Break::Auto,
            orphans: 2,
            widows: 2,
            column_count: 0,
            table_header: false,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_content: AlignContent::Stretch,
            row_gap: Dimension::ZERO,
            column_gap: Dimension::ZERO,
            align_self: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            order: 0,
            grid_auto_columns: TrackSize::Auto,
            grid_auto_rows: TrackSize::Auto,
            grid_auto_flow: GridAutoFlow::Row,
            grid_auto_flow_dense: false,
            justify_items: AlignItems::Stretch,
            grid_column: GridPlacement::default(),
            grid_row: GridPlacement::default(),
            grid_area: 0,
            justify_self: None,
        }
    }
}
