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
    /// True for the vertical writing modes.
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
/// A length: automatic, a fixed number of px, or a percentage of the containing block.
pub enum Dimension {
    #[default]
    /// Chosen by the layout.
    Auto,
    /// A fixed length.
    Length(Px),
    /// Hundredths of a percent (`5000` = 50%), resolved against a base length.
    Percent(i32),
}

impl Dimension {
    /// A zero length.
    pub const ZERO: Dimension = Dimension::Length(Px::ZERO);

    /// A fixed length in whole px.
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
    /// A block-level box, stacked in its container.
    Block,
    /// The box and its subtree take no space, paint nothing, and are not paginated. Its
    /// children are not laid out.
    None,
    /// An inline box, whose children join the surrounding line.
    Inline,
    /// An atomic box laid out inline.
    InlineBlock,
    /// A flex container.
    Flex,
    /// A grid container.
    Grid,
    /// A table.
    Table,
    /// A table row.
    TableRow,
    /// A table cell.
    TableCell,
}

impl Display {
    /// Participates in an inline formatting context rather than block flow.
    pub fn is_inline_level(self) -> bool {
        matches!(self, Display::Inline | Display::InlineBlock)
    }
}

/// How a table sizes its columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableLayout {
    /// Columns follow the cells' max-content widths.
    #[default]
    Auto,
    /// Columns follow the first row: a cell with a fixed width sets its column, and the remaining
    /// width is shared equally by the other columns. Spanning cells do not set widths.
    Fixed,
}

/// Whether inline content may wrap onto a new line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhiteSpace {
    /// Lines break at spaces when they run past the right edge.
    #[default]
    Normal,
    /// Inline content stays on one line and may overflow. Spaces are handled as in `Normal`.
    NoWrap,
    /// As `NoWrap`, and a newline in the text starts a new line. Runs of spaces are kept. Blank
    /// lines (two newlines in a row) are not kept. Tabs are not expanded.
    Pre,
    /// As `Pre`, but lines may still wrap at spaces when they run past the right edge.
    PreWrap,
}

/// Horizontal alignment of inline content within a line. `Start` is the left edge in LTR and the
/// right edge in RTL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// Takes the containing block's value. At the top of a tree the value is `Start`.
    #[default]
    Inherit,
    /// Start edge: left in LTR, right in RTL.
    Start,
    /// Centred.
    Center,
    /// End edge: right in LTR, left in RTL.
    End,
    /// Spreads each line to the full width by widening its spaces. The last line of a run stays
    /// at the start edge, as in CSS.
    Justify,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Inline direction of text, and of flex and grid flow.
pub enum Direction {
    /// Takes the parent's direction. At the top of a tree the direction is `Ltr`.
    #[default]
    Inherit,
    /// Left to right.
    Ltr,
    /// Right to left.
    Rtl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// How a box is positioned.
pub enum Position {
    #[default]
    /// In normal flow, not offset.
    Static,
    /// In normal flow, offset by its insets.
    Relative,
    /// Taken out of flow and placed by its insets.
    Absolute,
    /// Laid out as `Absolute`, but the containing block is the nearest positioned ancestor rather
    /// than the viewport, since there is no viewport here.
    Fixed,
    /// Offset like `Relative`. There is no scroll model, so it never sticks.
    Sticky,
}

impl Position {
    /// Taken out of normal flow: absolutely positioned, or fixed.
    pub fn is_out_of_flow(self) -> bool {
        matches!(self, Position::Absolute | Position::Fixed)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Whether `width` and `height` include padding and border.
pub enum BoxSizing {
    #[default]
    /// They do not include padding and border.
    ContentBox,
    /// They include padding and border.
    BorderBox,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// What happens to content outside the box.
pub enum Overflow {
    #[default]
    /// Not clipped.
    Visible,
    /// Clipped to the box.
    Hidden,
    /// Clipped, as `Hidden`. There is no scroll model.
    Scroll,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Whether the box floats, and to which side.
pub enum Float {
    #[default]
    /// Not floated.
    None,
    /// Floats to the left.
    Left,
    /// Floats to the right.
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Which floats the box moves below.
pub enum Clear {
    #[default]
    /// No clearance.
    None,
    /// Moves below left floats.
    Left,
    /// Moves below right floats.
    Right,
    /// Moves below all floats.
    Both,
}

/// Forced and avoided breaks for pagination and columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Break {
    #[default]
    /// No forced break.
    Auto,
    /// Keeps the box whole, if it fits on a page.
    Avoid,
    /// Always breaks at this point.
    Always,
    /// Break to the next page on the left (odd-numbered pages in the page sequence are right).
    Left,
    /// Breaks to the next page on the right.
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Main axis and direction of flex items.
pub enum FlexDirection {
    #[default]
    /// Main axis runs left to right.
    Row,
    /// Main axis runs right to left.
    RowReverse,
    /// Main axis runs top to bottom.
    Column,
    /// Main axis runs bottom to top.
    ColumnReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Whether flex items stay on one line or wrap.
pub enum FlexWrap {
    #[default]
    /// A single line.
    NoWrap,
    /// Lines wrap, in the cross direction.
    Wrap,
    /// Lines wrap, in reverse cross order.
    WrapReverse,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Distribution of items along the main axis.
pub enum JustifyContent {
    #[default]
    /// Packed at the start.
    FlexStart,
    /// Packed at the end.
    FlexEnd,
    /// Centred.
    Center,
    /// Free space between items.
    SpaceBetween,
    /// Half as much space at each end as between items.
    SpaceAround,
    /// Equal space everywhere, including the ends.
    SpaceEvenly,
}

/// Cross-axis (or, for grid, justify/align within a cell) alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    #[default]
    /// Stretched to fill the line.
    Stretch,
    /// Aligned to the start of the line.
    FlexStart,
    /// Aligned to the end of the line.
    FlexEnd,
    /// Centred in the line.
    Center,
    /// Aligned on the baseline.
    Baseline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Distribution of lines along the cross axis.
pub enum AlignContent {
    #[default]
    /// Lines stretch to fill the container.
    Stretch,
    /// Lines packed at the start.
    FlexStart,
    /// Lines packed at the end.
    FlexEnd,
    /// Lines centred.
    Center,
    /// Free space between lines.
    SpaceBetween,
    /// Half as much space at each end as between lines.
    SpaceAround,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Direction in which auto-placed grid items fill the grid.
pub enum GridAutoFlow {
    #[default]
    /// Items fill rows first.
    Row,
    /// Items fill columns first.
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
    MinMax {
        /// Lower bound.
        min: Dimension,
        /// Upper bound.
        max: Dimension,
    },
}

/// Explicit track list, in order. Tracks beyond `len` are implicit (`grid_auto_*`).
///
/// `line_names[i]` names grid line `i` (0-based: line 0 is before the first track). `0` means
/// unnamed. Lines are named with [`Tracks::with_line_name`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tracks {
    /// Track sizes. Entries past `len` are unused.
    pub items: [TrackSize; MAX_TRACKS],
    /// Number of tracks.
    pub len: u8,
    /// Name of the grid line before each track, or `0` for none. One more entry than tracks.
    pub line_names: [u8; MAX_TRACKS + 1],
}

impl Tracks {
    /// Panics if more than `MAX_TRACKS` tracks are given. Use [`Tracks::try_new`] to get an error.
    pub fn new(sizes: &[TrackSize]) -> Self {
        Self::try_new(sizes).expect("too many explicit tracks")
    }

    /// `repeat(count, pattern)`: the pattern written out `count` times. Returns `None` when the
    /// result would have more than `MAX_TRACKS` tracks.
    pub fn repeat(count: usize, pattern: &[TrackSize]) -> Option<Self> {
        let total = count.checked_mul(pattern.len())?;
        if total > MAX_TRACKS {
            return None;
        }
        let mut sizes = [TrackSize::Auto; MAX_TRACKS];
        for (index, size) in sizes.iter_mut().take(total).enumerate() {
            *size = pattern[index % pattern.len()];
        }
        Self::try_new(&sizes[..total])
    }

    /// Like [`Tracks::new`], but returns `None` instead of panicking when more than `MAX_TRACKS`
    /// tracks are given.
    pub fn try_new(sizes: &[TrackSize]) -> Option<Self> {
        if sizes.len() > MAX_TRACKS {
            return None;
        }
        let mut items = [TrackSize::Auto; MAX_TRACKS];
        items[..sizes.len()].copy_from_slice(sizes);
        Some(Tracks {
            items,
            len: sizes.len() as u8,
            line_names: [0; MAX_TRACKS + 1],
        })
    }

    /// Names the grid line before track `line` (0-based; `len` names the line after the last
    /// track). `name` is `1..=255`; `0` clears the name. Lines beyond the track list are ignored.
    pub fn with_line_name(mut self, line: usize, name: u8) -> Self {
        if line <= self.len as usize {
            self.line_names[line] = name;
        }
        self
    }

    /// The tracks in use.
    pub fn as_slice(&self) -> &[TrackSize] {
        &self.items[..self.len as usize]
    }
}

/// Grid container template: explicit tracks, line names and named areas. Kept on the node, not in
/// `BoxStyle`, so that styles stay small.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridTemplate {
    /// Column tracks.
    pub columns: Tracks,
    /// Row tracks.
    pub rows: Tracks,
    /// Row-major named areas. `0` is an empty cell.
    pub areas: [[u8; MAX_AREA_DIM]; MAX_AREA_DIM],
    /// `repeat(auto-fill, <px>)` for columns: as many columns of this fixed px width as fit the
    /// container, after any explicit columns. `None` when not used. Only fixed px widths are
    /// supported.
    pub auto_fill_columns: Option<Px>,
    /// With `auto_fill_columns`, `auto-fit` behaviour: repeated columns that no item occupies
    /// collapse to zero width. Ignored without `auto_fill_columns`.
    pub auto_fit: bool,
}

/// A grid line reference for one side of an item's placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridLine {
    #[default]
    /// Chosen by auto-placement.
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
    /// Where the item starts.
    pub start: GridLine,
    /// Where the item ends.
    pub end: GridLine,
}

/// Style for one box. `max_*` set to `Auto` means no maximum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxStyle {
    /// How the box takes part in layout.
    pub display: Display,
    /// Inline direction. Inherited when `Inherit`.
    pub direction: Direction,
    /// Alignment of inline content within each line box.
    pub text_align: TextAlign,
    /// Indent of the first line of an inline run that starts its block. Applied to inline runs
    /// only; block-level text leaves are not indented.
    pub text_indent: Dimension,
    /// Height of each line of an inline run. `auto` (the default) uses the natural height of the
    /// pieces. A length adds half-leading above and below them. Only lengths are supported.
    pub line_height: Dimension,
    /// Whether inline content wraps. See [`WhiteSpace`].
    pub white_space: WhiteSpace,
    /// Width divided by height for a box whose height is `auto`. Applies in block flow only, and
    /// only when the width is known. `None` or a non-positive value means no ratio.
    pub aspect_ratio: Option<f32>,
    /// Writing mode. Inherited when `Inherit`.
    pub writing_mode: WritingMode,
    /// Positioning scheme.
    pub position: Position,
    /// Whether sizes include padding and border.
    pub box_sizing: BoxSizing,
    /// Width.
    pub width: Dimension,
    /// Height.
    pub height: Dimension,
    /// Minimum width.
    pub min_width: Dimension,
    /// Maximum width.
    pub max_width: Dimension,
    /// Minimum height.
    pub min_height: Dimension,
    /// Maximum height.
    pub max_height: Dimension,
    /// `Auto` is only meaningful for horizontal margins.
    pub margin: Edges<Dimension>,
    /// Padding on each side.
    pub padding: Edges<Dimension>,
    /// Border width on each side.
    pub border: Edges<Px>,
    /// Offsets for `Relative` and `Absolute` boxes (top, right, bottom, left).
    pub inset: Edges<Dimension>,
    /// Clipping of content outside the box.
    pub overflow: Overflow,
    /// Paint order within the containing stacking order. See `paint::paint_order`.
    pub z_index: Option<i32>,
    /// Floating side.
    pub float: Float,
    /// Floats to clear below.
    pub clear: Clear,

    // Pagination and columns.
    /// Page-break rule before the box.
    pub break_before: Break,
    /// Page-break rule after the box.
    pub break_after: Break,
    /// Whether the box may be split across pages.
    pub break_inside: Break,
    /// Minimum lines at the bottom of a page before a break (text leaves).
    pub orphans: u16,
    /// Minimum lines at the top of a page after a break (text leaves).
    pub widows: u16,
    /// Number of columns; `0` or `1` means a single column.
    pub column_count: u16,
    /// Marks a table row to repeat at the top of each page it spans.
    pub table_header: bool,
    /// Number of table columns a cell spans. `0` counts as `1`.
    pub column_span: u16,
    /// Number of table rows a cell spans. `0` counts as `1`.
    pub row_span: u16,
    /// How a table sizes its columns. See [`TableLayout`].
    pub table_layout: TableLayout,

    // Flex container.
    /// Main axis and direction of a flex container.
    pub flex_direction: FlexDirection,
    /// Whether flex items wrap.
    pub flex_wrap: FlexWrap,
    /// Distribution of items along the main axis.
    pub justify_content: JustifyContent,
    /// Alignment of items across the cross axis.
    pub align_items: AlignItems,
    /// Distribution of lines across the cross axis.
    pub align_content: AlignContent,
    /// Gap between rows.
    pub row_gap: Dimension,
    /// Gap between columns.
    pub column_gap: Dimension,

    // Flex item.
    /// `None` means `auto` (the stretch-to-fit default is `align_items`).
    pub align_self: Option<AlignItems>,
    /// Share of free space taken when growing.
    pub flex_grow: f32,
    /// Share of overflow taken when shrinking.
    pub flex_shrink: f32,
    /// Starting size before growing and shrinking.
    pub flex_basis: Dimension,
    /// Order among siblings. Lower values come first.
    pub order: i32,

    // Grid container templates live on the node (see `GridTemplate`) to keep styles small.
    /// Size of implicit columns.
    pub grid_auto_columns: TrackSize,
    /// Size of implicit rows.
    pub grid_auto_rows: TrackSize,
    /// Direction of auto-placement.
    pub grid_auto_flow: GridAutoFlow,
    /// Whether auto-placement fills gaps left by earlier items.
    pub grid_auto_flow_dense: bool,
    /// Default justification of items within their cells.
    pub justify_items: AlignItems,

    // Grid item.
    /// Column placement.
    pub grid_column: GridPlacement,
    /// Row placement.
    pub grid_row: GridPlacement,
    /// Named area id (from the parent grid's template areas). `0` for none.
    pub grid_area: u8,
    /// Justification of this item. `None` uses `justify_items`.
    pub justify_self: Option<AlignItems>,
}

impl Default for BoxStyle {
    fn default() -> Self {
        BoxStyle {
            display: Display::Block,
            direction: Direction::Inherit,
            text_align: TextAlign::Inherit,
            text_indent: Dimension::Length(Px::ZERO),
            line_height: Dimension::Auto,
            white_space: WhiteSpace::Normal,
            aspect_ratio: None,
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
            column_span: 1,
            row_span: 1,
            table_layout: TableLayout::Auto,
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
