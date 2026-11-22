//! Tables in the Markdown (`story:page-text-tables`, `docs/design/page-as-text.md` section 4):
//! an HTML table as a GitHub Flavored Markdown table, its slots placed by the HTML Standard's
//! table processing model (`colspan`, `rowspan`), the tables nested in its cells after it.

use std::collections::HashMap;
use std::rc::Rc;

use surf_html::{Namespace, NodeData, NodeId, Tree};

use crate::count_table_visit;

use super::{CUT, Frame, Para, TEXT_CHARS_MAX, Walker, Word, attribute, count_node_visit};

/// The columns a table shows; a wider one gets one more column, `[cut]`.
pub const COLUMNS_MAX: usize = 20;

/// The most columns a cell spans (the HTML Standard's limit for `colspan`).
const COLSPAN_MAX: usize = 1_000;

/// The most rows a cell spans (the HTML Standard's limit for `rowspan`).
const ROWSPAN_MAX: usize = 65_534;

/// What a slot of the grid holds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Slot {
    Empty,
    /// The cell whose top left slot this is.
    Cell(NodeId),
    /// A slot another cell spans.
    Spanned,
}

/// The local name of an element in the HTML namespace.
fn html_name(tree: &Tree, node: NodeId) -> Option<&str> {
    match tree.data(node) {
        NodeData::Element {
            namespace: Namespace::Html,
            local_name,
            ..
        } => Some(local_name),
        _ => None,
    }
}

/// A span attribute: the HTML Standard's rules for parsing non-negative integers, `default`
/// when absent or not a number, at most `max`.
fn span(value: Option<&str>, default: usize, max: usize) -> usize {
    let Some(value) = value else {
        return default;
    };
    let digits = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let digits = digits.strip_prefix('+').unwrap_or(digits);
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return default;
    }
    digits[..end].parse::<usize>().map_or(max, |n| n.min(max))
}

impl Walker<'_> {
    /// Write the table `table` and, after it, every table nested in it, numbered from the
    /// document's count in document order.
    pub(super) fn write_tables(&mut self, table: NodeId) {
        let nested = self.number_nested(table);
        self.write_table(table, &nested);
        let mut order: Vec<(usize, NodeId)> = nested.iter().map(|(&node, &n)| (n, node)).collect();
        order.sort_unstable();
        for (number, node) in order {
            self.block_boundary();
            self.emit(&format!("[table {number}]"));
            self.block_boundary();
            self.write_table(node, &nested);
        }
    }

    /// The tables inside `table`'s subtree, hidden ones left out, each with its number.
    fn number_nested(&mut self, table: NodeId) -> Rc<HashMap<NodeId, usize>> {
        let tree = self.tree;
        let mut numbers = HashMap::new();
        let mut pending: Vec<NodeId> = tree.children(table).iter().rev().copied().collect();
        while let Some(node) = pending.pop() {
            count_node_visit();
            if self.facts.hidden.contains(&node) {
                continue;
            }
            match html_name(tree, node) {
                Some("svg" | "math") => continue,
                Some("table") => {
                    self.tables_seen += 1;
                    numbers.insert(node, self.tables_seen);
                }
                _ => {}
            }
            pending.extend(tree.children(node).iter().rev());
        }
        Rc::new(numbers)
    }

    /// The inline content of `node` as one paragraph, its blocks joined by a space, the tables
    /// of `numbers` written `[table <n>]`; at most `budget` characters of page text.
    fn inline(
        &self,
        node: NodeId,
        numbers: &Rc<HashMap<NodeId, usize>>,
        budget: Option<usize>,
    ) -> Para {
        let mut walk = Walker::new(self.tree, Rc::clone(&self.facts), None);
        walk.stack = self
            .tree
            .children(node)
            .iter()
            .rev()
            .map(|&c| Frame::Enter(c))
            .collect();
        walk.numbers = Rc::clone(numbers);
        walk.flatten = 1;
        walk.para.budget = budget;
        while let Some(frame) = walk.stack.pop() {
            match frame {
                Frame::Enter(child) => walk.enter(child),
                Frame::Exit(child, close) => walk.exit(child, close),
            }
        }
        walk.para
    }

    /// The rows of `table` in their order: those of `thead`, then those of `tbody` and directly
    /// in the table in tree order, then those of `tfoot`; each with its row group, which ends a
    /// `rowspan`. Hidden ones left out.
    fn rows(&self, table: NodeId) -> (Vec<(NodeId, NodeId)>, bool) {
        let tree = self.tree;
        let visible = |node: &NodeId| !self.facts.hidden.contains(node);
        let rows_of = |group: NodeId| {
            tree.children(group)
                .iter()
                .copied()
                .filter(visible)
                .filter(|&r| html_name(tree, r) == Some("tr"))
                .map(move |r| (r, group))
                .collect::<Vec<_>>()
        };
        let (mut head, mut body, mut foot) = (Vec::new(), Vec::new(), Vec::new());
        for child in tree.children(table).iter().copied().filter(visible) {
            match html_name(tree, child) {
                Some("thead") => head.extend(rows_of(child)),
                Some("tbody") => body.extend(rows_of(child)),
                Some("tfoot") => foot.extend(rows_of(child)),
                Some("tr") => body.push((child, table)),
                _ => {}
            }
        }
        let has_head = !head.is_empty();
        head.extend(body);
        head.extend(foot);
        (head, has_head)
    }

    fn cells(&self, row: NodeId) -> Vec<NodeId> {
        let tree = self.tree;
        tree.children(row)
            .iter()
            .copied()
            .filter(|c| !self.facts.hidden.contains(c))
            .filter(|&c| matches!(html_name(tree, c), Some("td" | "th")))
            .collect()
    }

    /// Write one table: its captions as paragraphs (the first `caption` child is the table's,
    /// a later one is written after it, in tree order), then the header row, the delimiter row
    /// and the other rows.
    fn write_table(&mut self, table: NodeId, numbers: &Rc<HashMap<NodeId, usize>>) {
        let tree = self.tree;
        let captions: Vec<NodeId> = tree
            .children(table)
            .iter()
            .copied()
            .filter(|c| !self.facts.hidden.contains(c) && html_name(tree, *c) == Some("caption"))
            .collect();
        for caption in captions {
            for line in self.inline(caption, numbers, None).lines(self.text_width()) {
                self.emit(&line);
            }
            self.block_boundary();
        }

        let (rows, has_head) = self.rows(table);
        let cells: Vec<Vec<NodeId>> = rows.iter().map(|&(row, _)| self.cells(row)).collect();
        let header_row = has_head
            || cells.first().is_some_and(|first| {
                !first.is_empty() && first.iter().all(|&c| html_name(tree, c) == Some("th"))
            });

        // The rows after each row in its row group, counted once from the last row.
        let mut left_in_group = vec![0usize; rows.len()];
        for index in (0..rows.len().saturating_sub(1)).rev() {
            count_table_visit();
            if rows[index + 1].1 == rows[index].1 {
                left_in_group[index] = left_in_group[index + 1] + 1;
            }
        }

        // The slots of each row, for the first `COLUMNS_MAX + 1` columns.
        let track = COLUMNS_MAX + 1;
        let mut covered = vec![0usize; track];
        let mut width = 0usize;
        let mut grid: Vec<Vec<Slot>> = Vec::with_capacity(rows.len());
        for (index, (&(_, group), row_cells)) in rows.iter().zip(&cells).enumerate() {
            if index > 0 && rows[index - 1].1 != group {
                covered.iter_mut().for_each(|c| *c = 0);
            }
            count_table_visit();
            let left_in_group = left_in_group[index];
            let mut slots = vec![Slot::Empty; track];
            for (slot, rows_left) in slots.iter_mut().zip(covered.iter_mut()) {
                if *rows_left > 0 {
                    *slot = Slot::Spanned;
                    *rows_left -= 1;
                }
            }
            let mut column = 0usize;
            for &cell in row_cells {
                count_table_visit();
                while column < track && slots[column] != Slot::Empty {
                    column += 1;
                }
                let attributes = match tree.data(cell) {
                    NodeData::Element { attributes, .. } => attributes.as_slice(),
                    _ => &[],
                };
                let colspan = span(attribute(attributes, "colspan"), 1, COLSPAN_MAX).max(1);
                let rowspan = match span(attribute(attributes, "rowspan"), 1, ROWSPAN_MAX) {
                    0 => left_in_group + 1,
                    n => n.min(left_in_group + 1),
                };
                if column < track {
                    slots[column] = Slot::Cell(cell);
                    for slot in &mut slots[column + 1..(column + colspan).min(track)] {
                        *slot = Slot::Spanned;
                    }
                    for rows_left in &mut covered[column..(column + colspan).min(track)] {
                        *rows_left = (*rows_left).max(rowspan - 1);
                    }
                }
                column = column.saturating_add(colspan);
                width = width.max(column);
            }
            if let Some(last) = slots.iter().rposition(|s| *s != Slot::Empty) {
                width = width.max(last + 1);
            }
            grid.push(slots);
        }
        if width == 0 {
            return;
        }
        let cut = width > COLUMNS_MAX;
        let shown = width.min(COLUMNS_MAX);

        let mut texts: HashMap<NodeId, String> = HashMap::new();
        let mut line_of = |walker: &Self, slots: Option<&Vec<Slot>>| {
            let mut parts: Vec<String> = Vec::with_capacity(shown + 1);
            for column in 0..shown {
                let text = match slots.map(|s| s[column]) {
                    Some(Slot::Cell(cell)) => texts
                        .entry(cell)
                        .or_insert_with(|| {
                            walker
                                .inline(cell, numbers, Some(TEXT_CHARS_MAX))
                                .words()
                                .iter()
                                .map(Word::joined)
                                .collect::<Vec<_>>()
                                .join(" ")
                        })
                        .clone(),
                    _ => String::new(),
                };
                parts.push(text);
            }
            if cut {
                parts.push(CUT.to_string());
            }
            format!("| {} |", parts.join(" | "))
        };
        let mut lines = Vec::with_capacity(grid.len() + 2);
        let body = if header_row {
            lines.push(line_of(self, grid.first()));
            &grid[1..]
        } else {
            lines.push(line_of(self, None));
            &grid[..]
        };
        lines.push(format!(
            "| {} |",
            vec!["---"; shown + usize::from(cut)].join(" | ")
        ));
        for slots in body {
            lines.push(line_of(self, Some(slots)));
        }
        for line in lines {
            self.emit(&line);
        }
    }
}
