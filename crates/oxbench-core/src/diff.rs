//! Line-by-line text comparison, with word or character highlights inside changed lines.

use std::ops::Range;
use std::time::{Duration, Instant};

use similar::{Algorithm, ChangeTag, DiffOp, TextDiff, capture_diff_slices_by_key_deadline};

/// Lines kept around each change when unchanged stretches are folded, as `diff -u` does.
pub const DEFAULT_CONTEXT_LINES: usize = 3;

/// After this, the line diff settles for a correct but less minimal result.
const LINE_DIFF_TIMEOUT: Duration = Duration::from_secs(2);
const INLINE_DIFF_TIMEOUT: Duration = Duration::from_millis(20);
/// Longer lines are highlighted as a whole: diffing inside them is too slow to redo on every keystroke.
const MAX_INLINE_DIFF_BYTES: usize = 10_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InlineGranularity {
    #[default]
    Word,
    Character,
}

impl InlineGranularity {
    pub const ALL: [InlineGranularity; 2] = [InlineGranularity::Word, InlineGranularity::Character];

    pub const fn label(self) -> &'static str {
        match self {
            InlineGranularity::Word => "Words",
            InlineGranularity::Character => "Characters",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompareOptions {
    pub should_ignore_case: bool,
    /// Treats runs of spaces and tabs as one space, and ignores them at the ends of lines.
    pub should_ignore_whitespace: bool,
    pub inline_granularity: InlineGranularity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Equal,
    /// The line exists on both sides with different text.
    Changed,
    Removed,
    Added,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    /// 1-based, as editors show it.
    pub number: usize,
    pub text: String,
    /// Byte ranges of `text` that differ from the other side; empty for whole-line changes.
    pub highlights: Vec<Range<usize>>,
}

/// One row of a side-by-side view. `left` is missing for added lines, `right` for removed ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    pub kind: RowKind,
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub unchanged: usize,
}

impl DiffStats {
    pub const fn has_differences(self) -> bool {
        self.added + self.removed + self.changed > 0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    pub rows: Vec<DiffRow>,
    pub stats: DiffStats,
}

/// What a view shows: a row, or a fold standing in for unchanged rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayItem {
    Row(usize),
    Folded { count: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// One line of a unified view: which side of which row to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnifiedItem {
    Line { row: usize, side: Side },
    Folded { count: usize },
}

/// Compares two texts line by line and pairs up changed lines for side-by-side display.
pub fn compare_texts(left: &str, right: &str, options: CompareOptions) -> Comparison {
    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();
    let deadline = Instant::now() + LINE_DIFF_TIMEOUT;
    let ops = capture_diff_slices_by_key_deadline(
        Algorithm::Myers,
        &left_lines,
        &right_lines,
        |line| comparison_key(line, options),
        Some(deadline),
    );
    let mut builder = RowBuilder {
        left_lines: &left_lines,
        right_lines: &right_lines,
        granularity: options.inline_granularity,
        comparison: Comparison::default(),
    };
    for op in &ops {
        builder.add_op(op);
    }
    builder.comparison
}

/// Folds unchanged stretches, keeping `context` rows on each side of every change.
pub fn fold_unchanged(rows: &[DiffRow], context: usize) -> Vec<DisplayItem> {
    let mut items = Vec::new();
    let mut next_row = 0;
    for hunk in find_hunks(rows, context) {
        if hunk.start > next_row {
            items.push(DisplayItem::Folded {
                count: hunk.start - next_row,
            });
        }
        items.extend(hunk.clone().map(DisplayItem::Row));
        next_row = hunk.end;
    }
    if next_row < rows.len() {
        items.push(DisplayItem::Folded {
            count: rows.len() - next_row,
        });
    }
    items
}

/// Every row, unfolded.
pub fn list_all_rows(rows: &[DiffRow]) -> Vec<DisplayItem> {
    (0..rows.len()).map(DisplayItem::Row).collect()
}

/// Orders items for a unified view: within each run of changes, all removed lines come first.
pub fn arrange_unified(rows: &[DiffRow], items: &[DisplayItem]) -> Vec<UnifiedItem> {
    let mut arranged = Vec::new();
    let mut pending_added = Vec::new();
    for item in items {
        let row_index = match *item {
            DisplayItem::Row(row_index) => row_index,
            DisplayItem::Folded { count } => {
                arranged.append(&mut pending_added);
                arranged.push(UnifiedItem::Folded { count });
                continue;
            }
        };
        let Some(row) = rows.get(row_index) else {
            continue;
        };
        if row.kind == RowKind::Equal {
            arranged.append(&mut pending_added);
            arranged.push(UnifiedItem::Line {
                row: row_index,
                side: Side::Left,
            });
            continue;
        }
        if row.left.is_some() {
            arranged.push(UnifiedItem::Line {
                row: row_index,
                side: Side::Left,
            });
        }
        if row.right.is_some() {
            pending_added.push(UnifiedItem::Line {
                row: row_index,
                side: Side::Right,
            });
        }
    }
    arranged.append(&mut pending_added);
    arranged
}

/// A patch in unified format, the same shape `diff -u` and `git diff` print.
/// Empty when there are no differences.
pub fn format_unified_diff(
    rows: &[DiffRow],
    left_name: &str,
    right_name: &str,
    context: usize,
) -> String {
    let hunks = find_hunks(rows, context);
    if hunks.is_empty() {
        return String::new();
    }
    let mut patch = format!("--- {left_name}\n+++ {right_name}\n");
    for hunk in hunks {
        let (Some(before), Some(hunk_rows)) = (rows.get(..hunk.start), rows.get(hunk)) else {
            continue;
        };
        let left_before = count_side(before, Side::Left);
        let right_before = count_side(before, Side::Right);
        patch.push_str("@@ -");
        patch.push_str(&format_hunk_range(
            left_before,
            count_side(hunk_rows, Side::Left),
        ));
        patch.push_str(" +");
        patch.push_str(&format_hunk_range(
            right_before,
            count_side(hunk_rows, Side::Right),
        ));
        patch.push_str(" @@\n");
        write_hunk_lines(&mut patch, hunk_rows);
    }
    patch
}

struct RowBuilder<'a> {
    left_lines: &'a [&'a str],
    right_lines: &'a [&'a str],
    granularity: InlineGranularity,
    comparison: Comparison,
}

impl RowBuilder<'_> {
    fn add_op(&mut self, op: &DiffOp) {
        match *op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for offset in 0..len {
                    self.push_row(
                        RowKind::Equal,
                        self.plain_left(old_index + offset),
                        self.plain_right(new_index + offset),
                    );
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => self.push_removed(old_index..old_index + old_len),
            DiffOp::Insert {
                new_index, new_len, ..
            } => self.push_added(new_index..new_index + new_len),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let paired = old_len.min(new_len);
                for offset in 0..paired {
                    self.push_changed(old_index + offset, new_index + offset);
                }
                self.push_removed(old_index + paired..old_index + old_len);
                self.push_added(new_index + paired..new_index + new_len);
            }
        }
    }

    fn push_removed(&mut self, indices: Range<usize>) {
        for index in indices {
            self.push_row(RowKind::Removed, self.plain_left(index), None);
        }
    }

    fn push_added(&mut self, indices: Range<usize>) {
        for index in indices {
            self.push_row(RowKind::Added, None, self.plain_right(index));
        }
    }

    fn push_changed(&mut self, left_index: usize, right_index: usize) {
        let (Some(left_text), Some(right_text)) = (
            self.left_lines.get(left_index),
            self.right_lines.get(right_index),
        ) else {
            return;
        };
        let (left_highlights, right_highlights) =
            find_inline_highlights(left_text, right_text, self.granularity);
        let left = make_line(left_index, left_text, left_highlights);
        let right = make_line(right_index, right_text, right_highlights);
        self.push_row(RowKind::Changed, Some(left), Some(right));
    }

    fn plain_left(&self, index: usize) -> Option<DiffLine> {
        let text = self.left_lines.get(index)?;
        Some(make_line(index, text, Vec::new()))
    }

    fn plain_right(&self, index: usize) -> Option<DiffLine> {
        let text = self.right_lines.get(index)?;
        Some(make_line(index, text, Vec::new()))
    }

    fn push_row(&mut self, kind: RowKind, left: Option<DiffLine>, right: Option<DiffLine>) {
        let stats = &mut self.comparison.stats;
        match kind {
            RowKind::Equal => stats.unchanged += 1,
            RowKind::Changed => stats.changed += 1,
            RowKind::Removed => stats.removed += 1,
            RowKind::Added => stats.added += 1,
        }
        self.comparison.rows.push(DiffRow { kind, left, right });
    }
}

fn make_line(index: usize, text: &str, highlights: Vec<Range<usize>>) -> DiffLine {
    DiffLine {
        number: index + 1,
        text: text.to_owned(),
        highlights,
    }
}

fn comparison_key(line: &str, options: CompareOptions) -> String {
    let key = if options.should_ignore_whitespace {
        line.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        line.to_owned()
    };
    if options.should_ignore_case {
        return key.to_lowercase();
    }
    key
}

fn find_inline_highlights(
    left: &str,
    right: &str,
    granularity: InlineGranularity,
) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    if left.len() > MAX_INLINE_DIFF_BYTES || right.len() > MAX_INLINE_DIFF_BYTES {
        return (whole_line(left), whole_line(right));
    }
    let mut config = TextDiff::configure();
    config.timeout(INLINE_DIFF_TIMEOUT);
    let diff = match granularity {
        InlineGranularity::Word => config.diff_words(left, right),
        InlineGranularity::Character => config.diff_chars(left, right),
    };
    let mut left_highlights = Vec::new();
    let mut right_highlights = Vec::new();
    let (mut left_offset, mut right_offset) = (0, 0);
    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Equal => {
                left_offset += len;
                right_offset += len;
            }
            ChangeTag::Delete => {
                push_merged(&mut left_highlights, left_offset..left_offset + len);
                left_offset += len;
            }
            ChangeTag::Insert => {
                push_merged(&mut right_highlights, right_offset..right_offset + len);
                right_offset += len;
            }
        }
    }
    (left_highlights, right_highlights)
}

fn whole_line(text: &str) -> Vec<Range<usize>> {
    if text.is_empty() {
        return Vec::new();
    }
    std::iter::once(0..text.len()).collect()
}

fn push_merged(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    if let Some(last) = ranges.last_mut()
        && last.end == range.start
    {
        last.end = range.end;
        return;
    }
    ranges.push(range);
}

/// Row ranges around each change, with `context` rows either side; ranges that touch are merged.
fn find_hunks(rows: &[DiffRow], context: usize) -> Vec<Range<usize>> {
    let mut hunks: Vec<Range<usize>> = Vec::new();
    let changed_rows = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.kind != RowKind::Equal);
    for (index, _) in changed_rows {
        let start = index.saturating_sub(context);
        let end = (index + 1 + context).min(rows.len());
        if let Some(last) = hunks.last_mut()
            && start <= last.end
        {
            last.end = end;
            continue;
        }
        hunks.push(start..end);
    }
    hunks
}

fn count_side(rows: &[DiffRow], side: Side) -> usize {
    rows.iter()
        .filter(|row| match side {
            Side::Left => row.left.is_some(),
            Side::Right => row.right.is_some(),
        })
        .count()
}

/// `start,count`, where an empty range starts at the line before it, as patch tools expect.
fn format_hunk_range(lines_before: usize, count: usize) -> String {
    let start = if count == 0 {
        lines_before
    } else {
        lines_before + 1
    };
    if count == 1 {
        return start.to_string();
    }
    format!("{start},{count}")
}

fn write_hunk_lines(patch: &mut String, rows: &[DiffRow]) {
    let mut pending_added: Vec<&str> = Vec::new();
    for row in rows {
        if row.kind == RowKind::Equal {
            write_patch_lines(patch, '+', &mut pending_added);
            if let Some(line) = &row.left {
                write_patch_line(patch, ' ', &line.text);
            }
            continue;
        }
        if let Some(line) = &row.left {
            write_patch_line(patch, '-', &line.text);
        }
        if let Some(line) = &row.right {
            pending_added.push(&line.text);
        }
    }
    write_patch_lines(patch, '+', &mut pending_added);
}

fn write_patch_lines(patch: &mut String, marker: char, lines: &mut Vec<&str>) {
    for line in lines.drain(..) {
        write_patch_line(patch, marker, line);
    }
}

fn write_patch_line(patch: &mut String, marker: char, text: &str) {
    patch.push(marker);
    patch.push_str(text);
    patch.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(left: &str, right: &str) -> Comparison {
        compare_texts(left, right, CompareOptions::default())
    }

    fn kinds(comparison: &Comparison) -> Vec<RowKind> {
        comparison.rows.iter().map(|row| row.kind).collect()
    }

    fn highlighted(line: &DiffLine) -> Vec<&str> {
        line.highlights
            .iter()
            .map(|range| &line.text[range.clone()])
            .collect()
    }

    #[test]
    fn identical_texts_have_no_differences() {
        let comparison = compare("one\ntwo\n", "one\ntwo");
        assert!(!comparison.stats.has_differences());
        assert_eq!(kinds(&comparison), [RowKind::Equal, RowKind::Equal]);
        assert_eq!(comparison.stats.unchanged, 2);
    }

    #[test]
    fn empty_texts_have_no_rows() {
        let comparison = compare("", "");
        assert_eq!(comparison.rows, []);
        assert!(!comparison.stats.has_differences());
    }

    #[test]
    fn text_on_one_side_only_is_all_added() {
        let comparison = compare("", "a\nb");
        assert_eq!(kinds(&comparison), [RowKind::Added, RowKind::Added]);
        assert_eq!(comparison.rows[1].right.as_ref().unwrap().number, 2);
        assert!(comparison.rows[1].left.is_none());
    }

    #[test]
    fn removed_and_added_lines_keep_their_own_line_numbers() {
        let comparison = compare("a\nb\nc", "a\nc\nd");
        assert_eq!(
            kinds(&comparison),
            [
                RowKind::Equal,
                RowKind::Removed,
                RowKind::Equal,
                RowKind::Added
            ]
        );
        assert_eq!(comparison.rows[1].left.as_ref().unwrap().number, 2);
        let last = comparison.rows[2].right.as_ref().unwrap();
        assert_eq!((last.number, last.text.as_str()), (2, "c"));
        assert_eq!(comparison.rows[3].right.as_ref().unwrap().number, 3);
        assert_eq!(
            comparison.stats,
            DiffStats {
                added: 1,
                removed: 1,
                changed: 0,
                unchanged: 2
            }
        );
    }

    #[test]
    fn changed_lines_highlight_the_words_that_differ() {
        let comparison = compare("the quick brown fox", "the slow brown fox");
        let row = &comparison.rows[0];
        assert_eq!(row.kind, RowKind::Changed);
        assert_eq!(highlighted(row.left.as_ref().unwrap()), ["quick"]);
        assert_eq!(highlighted(row.right.as_ref().unwrap()), ["slow"]);
    }

    #[test]
    fn character_highlights_mark_single_letters() {
        let options = CompareOptions {
            inline_granularity: InlineGranularity::Character,
            ..CompareOptions::default()
        };
        let comparison = compare_texts("color", "colour", options);
        let row = &comparison.rows[0];
        assert_eq!(highlighted(row.left.as_ref().unwrap()), Vec::<&str>::new());
        assert_eq!(highlighted(row.right.as_ref().unwrap()), ["u"]);
    }

    #[test]
    fn character_highlights_stay_on_character_boundaries() {
        let options = CompareOptions {
            inline_granularity: InlineGranularity::Character,
            ..CompareOptions::default()
        };
        let comparison = compare_texts("café au lait", "cafe au lait", options);
        let row = &comparison.rows[0];
        assert_eq!(highlighted(row.left.as_ref().unwrap()), ["é"]);
        assert_eq!(highlighted(row.right.as_ref().unwrap()), ["e"]);
    }

    #[test]
    fn uneven_replacements_pair_lines_then_add_the_rest() {
        let comparison = compare("a\nold one\nz", "a\nnew one\nnew two\nz");
        assert_eq!(
            kinds(&comparison),
            [
                RowKind::Equal,
                RowKind::Changed,
                RowKind::Added,
                RowKind::Equal
            ]
        );
    }

    #[test]
    fn very_long_changed_lines_are_highlighted_whole() {
        let left = "a".repeat(MAX_INLINE_DIFF_BYTES + 1);
        let right = format!("{left}b");
        let comparison = compare(&left, &right);
        let row = &comparison.rows[0];
        assert_eq!(highlighted(row.left.as_ref().unwrap()), [left.as_str()]);
        assert_eq!(highlighted(row.right.as_ref().unwrap()), [right.as_str()]);
    }

    #[test]
    fn ignoring_case_treats_case_changes_as_equal() {
        let options = CompareOptions {
            should_ignore_case: true,
            ..CompareOptions::default()
        };
        let comparison = compare_texts("Hello World", "hello world", options);
        assert!(!comparison.stats.has_differences());
        let row = &comparison.rows[0];
        assert_eq!(row.right.as_ref().unwrap().text, "hello world");
    }

    #[test]
    fn ignoring_whitespace_treats_spacing_changes_as_equal() {
        let options = CompareOptions {
            should_ignore_whitespace: true,
            ..CompareOptions::default()
        };
        let comparison = compare_texts("let  x =\t1;  ", "let x = 1;", options);
        assert!(!comparison.stats.has_differences());
        assert!(compare_texts("ab", "a b", options).stats.has_differences());
    }

    #[test]
    fn folding_keeps_context_around_each_change() {
        let left = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
        let right = "1\n2\n3\n4\n5\nsix\n7\n8\n9\n10";
        let comparison = compare(left, right);
        assert_eq!(
            fold_unchanged(&comparison.rows, 1),
            [
                DisplayItem::Folded { count: 4 },
                DisplayItem::Row(4),
                DisplayItem::Row(5),
                DisplayItem::Row(6),
                DisplayItem::Folded { count: 3 },
            ]
        );
    }

    #[test]
    fn folding_merges_changes_whose_context_touches() {
        let comparison = compare("a\nb\nc\nd\ne", "A\nb\nc\nd\nE");
        assert_eq!(
            fold_unchanged(&comparison.rows, 1),
            [
                DisplayItem::Row(0),
                DisplayItem::Row(1),
                DisplayItem::Folded { count: 1 },
                DisplayItem::Row(3),
                DisplayItem::Row(4),
            ]
        );
        assert_eq!(
            fold_unchanged(&comparison.rows, 2),
            list_all_rows(&comparison.rows)
        );
    }

    #[test]
    fn folding_identical_texts_folds_everything() {
        let comparison = compare("a\nb", "a\nb");
        assert_eq!(
            fold_unchanged(&comparison.rows, 3),
            [DisplayItem::Folded { count: 2 }]
        );
    }

    #[test]
    fn unified_view_lists_removed_lines_before_added_ones() {
        let comparison = compare("keep\nold 1\nold 2\nkeep", "keep\nnew 1\nnew 2\nkeep");
        let items = list_all_rows(&comparison.rows);
        let line = |row, side| UnifiedItem::Line { row, side };
        assert_eq!(
            arrange_unified(&comparison.rows, &items),
            [
                line(0, Side::Left),
                line(1, Side::Left),
                line(2, Side::Left),
                line(1, Side::Right),
                line(2, Side::Right),
                line(3, Side::Left),
            ]
        );
    }

    #[test]
    fn unified_patch_matches_diff_u() {
        let left = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
        let right = "1\n2\n3\n4\nfive\n6\n7\n8\n9\n10\n11\n";
        let comparison = compare(left, right);
        let patch = format_unified_diff(&comparison.rows, "a.txt", "b.txt", 3);
        let expected = "\
--- a.txt
+++ b.txt
@@ -2,9 +2,10 @@
 2
 3
 4
-5
+five
 6
 7
 8
 9
 10
+11
";
        assert_eq!(patch, expected);
    }

    #[test]
    fn unified_patch_ranges_for_pure_insertions_start_before_them() {
        let comparison = compare("", "a\nb");
        let patch = format_unified_diff(&comparison.rows, "a", "b", 3);
        assert_eq!(patch, "--- a\n+++ b\n@@ -0,0 +1,2 @@\n+a\n+b\n");
        let comparison = compare("x\ny", "x");
        let patch = format_unified_diff(&comparison.rows, "a", "b", 0);
        assert_eq!(patch, "--- a\n+++ b\n@@ -2 +1,0 @@\n-y\n");
    }

    #[test]
    fn unified_patch_is_empty_without_differences() {
        let comparison = compare("same", "same");
        assert_eq!(format_unified_diff(&comparison.rows, "a", "b", 3), "");
    }
}
