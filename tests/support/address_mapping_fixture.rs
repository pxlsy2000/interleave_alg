#[derive(Clone, Copy)]
enum LocalRows<'a> {
    PreserveHigh,
    Explicit(&'a [u64]),
}

struct MappingFixture<'a> {
    address_width: u8,
    granule_bytes: u64,
    target_rows: &'a [u64],
    local_rows: LocalRows<'a>,
}

impl MappingFixture<'_> {
    fn source(&self) -> String {
        let target_count = 1_u32 << self.target_rows.len();
        let mut remaining_granule = self.granule_bytes;
        let mut granule_bits = 0_u8;
        while remaining_granule > 1 {
            remaining_granule >>= 1;
            granule_bits = granule_bits.saturating_add(1);
        }
        let line_bits = self.address_width - granule_bits;
        let mut source = format!(
            "schema_version: 1\nname: address-test\naddress: {{ width_bits: {}, granule_bytes: {} }}\ntargets: {{ count: {target_count} }}\nmapping:\n  m:\n",
            self.address_width, self.granule_bytes
        );
        append_rows(&mut source, self.target_rows, line_bits);
        match self.local_rows {
            LocalRows::PreserveHigh => source.push_str("  l: { mode: preserve_high }\n"),
            LocalRows::Explicit(rows) => {
                source.push_str("  l:\n    mode: explicit\n");
                append_rows(&mut source, rows, line_bits);
            }
        }
        source
    }
}

fn append_rows(source: &mut String, rows: &[u64], line_bits: u8) {
    if rows.is_empty() {
        source.push_str("    rows: []\n");
        return;
    }
    source.push_str("    rows:\n");
    for row in rows {
        source.push_str("      - [");
        let mut separator = "";
        for bit in 0..line_bits {
            if row & (1_u64 << bit) != 0 {
                source.push_str(separator);
                source.push_str(&bit.to_string());
                separator = ", ";
            }
        }
        source.push_str("]\n");
    }
}

fn natural_fixture(
    address_width: u8,
    granule_bytes: u64,
    target_bits: u8,
) -> TestResult<MappingFixture<'static>> {
    const ROWS: [u64; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
    let target_rows = ROWS
        .get(..usize::from(target_bits))
        .ok_or_else(|| "natural fixture exceeds eight Target bits".to_owned())?;
    Ok(MappingFixture {
        address_width,
        granule_bytes,
        target_rows,
        local_rows: LocalRows::PreserveHigh,
    })
}

fn assert_row(
    row: &MappedAddress,
    expected: (&str, u64, u64, u16, u64, u64),
) -> TestResult {
    let parsed = address(expected.0)?;
    assert_eq!(row.input_address(), parsed);
    assert_eq!(row.line_address(), expected.1);
    assert_eq!(row.byte_offset(), expected.2);
    assert_eq!(row.target(), expected.3);
    assert_eq!(row.local_line_address(), expected.4);
    assert_eq!(row.local_byte_address(), expected.5);
    Ok(())
}
