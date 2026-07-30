use super::model::{LocalAddressRows, MappingModel, XorRow};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BinaryMatrix {
    columns: u8,
    rows: Vec<u64>,
}

impl BinaryMatrix {
    pub(super) fn from_rows(rows: &[XorRow], columns: u8) -> Self {
        Self {
            columns,
            rows: rows.iter().map(row_mask).collect(),
        }
    }

    pub(super) fn preserve_high(target_bits: u8, local_bits: u8, columns: u8) -> Self {
        let rows = (0..local_bits)
            .map(|local_bit| bit(target_bits + local_bit))
            .collect();
        Self { columns, rows }
    }

    pub(super) fn stacked(upper: &Self, lower: &Self) -> Self {
        let mut rows = Vec::with_capacity(upper.rows.len() + lower.rows.len());
        rows.extend_from_slice(&upper.rows);
        rows.extend_from_slice(&lower.rows);
        Self {
            columns: upper.columns,
            rows,
        }
    }

    pub(super) fn low_columns(&self, columns: u8) -> Self {
        let low_mask = match columns {
            0 => 0,
            64 => u64::MAX,
            width => bit(width) - 1,
        };
        Self {
            columns,
            rows: self.rows.iter().map(|row| row & low_mask).collect(),
        }
    }

    pub(super) fn rank(&self) -> u8 {
        let mut reduced = self.rows.clone();
        let mut rank = 0_u8;
        for column in 0..self.columns {
            let pivot_row = usize::from(rank);
            let pivot = reduced
                .iter()
                .enumerate()
                .skip(pivot_row)
                .find_map(|(index, row)| (row & bit(column) != 0).then_some(index));
            let Some(pivot_index) = pivot else {
                continue;
            };
            reduced.swap(pivot_row, pivot_index);
            let Some(pivot_mask) = reduced.get(pivot_row).copied() else {
                continue;
            };
            for row in reduced.iter_mut().skip(pivot_row.saturating_add(1)) {
                if *row & bit(column) != 0 {
                    *row ^= pivot_mask;
                }
            }
            rank += 1;
            if usize::from(rank) == reduced.len() {
                break;
            }
        }
        rank
    }
}

#[derive(Debug)]
pub(super) struct MappingMatrices {
    pub(super) target: BinaryMatrix,
    pub(super) local: BinaryMatrix,
    pub(super) combined: BinaryMatrix,
    pub(super) target_low: BinaryMatrix,
    preserve_high: BinaryMatrix,
}

impl MappingMatrices {
    pub(super) fn build(mapping: &MappingModel) -> Self {
        let target = BinaryMatrix::from_rows(&mapping.target_rows, mapping.line_bits);
        let preserve_high = BinaryMatrix::preserve_high(
            mapping.target_bits,
            mapping.local_address_bits,
            mapping.line_bits,
        );
        let local = match &mapping.local_rows {
            LocalAddressRows::PreserveHigh => preserve_high.clone(),
            LocalAddressRows::Explicit(rows) => BinaryMatrix::from_rows(rows, mapping.line_bits),
        };
        let combined = BinaryMatrix::stacked(&target, &local);
        let target_low = target.low_columns(mapping.target_bits);
        Self {
            target,
            local,
            combined,
            target_low,
            preserve_high,
        }
    }

    pub(super) fn local_matches_preserve_high(&self) -> bool {
        self.local == self.preserve_high
    }
}

fn row_mask(row: &XorRow) -> u64 {
    row.taps()
        .iter()
        .fold(0_u64, |mask, tap| mask | bit(tap.get()))
}

const fn bit(position: u8) -> u64 {
    1_u64 << position
}
