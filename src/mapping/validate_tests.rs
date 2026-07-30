use super::{natural_message, non_natural_path};

#[test]
fn rank_only_non_natural_contract_uses_target_rows_path() {
    // Given
    let low_rank_passes = false;
    let l_matches = true;

    // When
    let path = non_natural_path(low_rank_passes, l_matches);
    let message = natural_message(0, 1, l_matches);

    // Then
    assert_eq!(path.as_str(), "mapping.m.rows");
    assert_eq!(message, "rank(Mp)=0, expected 1");
}
