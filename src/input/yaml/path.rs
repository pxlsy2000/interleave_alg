use crate::issue::IssuePath;

pub(super) fn child_path(parent: &IssuePath, key: &str) -> IssuePath {
    if is_known_field(key) {
        parent.clone().field(key)
    } else {
        parent.clone().raw_key(key)
    }
}

fn is_known_field(key: &str) -> bool {
    matches!(
        key,
        "schema_version"
            | "name"
            | "address"
            | "width_bits"
            | "granule_bytes"
            | "targets"
            | "count"
            | "mapping"
            | "m"
            | "l"
            | "rows"
            | "mode"
            | "defaults"
            | "accesses"
            | "window_sizes"
            | "cases"
            | "enabled"
            | "kind"
            | "base_bytes"
            | "stride_bytes"
            | "schedule"
            | "streams"
    )
}
