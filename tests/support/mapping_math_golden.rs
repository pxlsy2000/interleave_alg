#[test]
fn emits_exact_check_array_for_every_classification() -> TestResult {
    // Given
    let cases = [
        GoldenCase {
            source: mapping_source(3, 1, &[1], None),
            classification: MappingClassification::ValidNatural,
            ranks: [1, 3, 1],
            l_matches: true,
            statuses: ["pass", "pass", "pass"],
            messages: [
                "all targets are reachable",
                "mapping is bijective",
                "local address is naturally ordered",
            ],
        },
        GoldenCase {
            source: mapping_source(3, 1, &[2], Some(&[1, 4])),
            classification: MappingClassification::ValidNonNatural,
            ranks: [1, 3, 0],
            l_matches: false,
            statuses: ["pass", "pass", "warning"],
            messages: [
                "all targets are reachable",
                "mapping is bijective",
                "rank(Mp)=0, expected 1; L != [0 I]",
            ],
        },
        GoldenCase {
            source: mapping_source(3, 1, &[0], None),
            classification: MappingClassification::InvalidTargetUnreachable,
            ranks: [0, 2, 0],
            l_matches: true,
            statuses: ["fail", "fail", "fail"],
            messages: [
                "rank(M)=0, expected 1",
                "rank(F)=2, expected 3",
                "rank(Mp)=0, expected 1",
            ],
        },
        GoldenCase {
            source: mapping_source(3, 1, &[1], Some(&[1, 4])),
            classification: MappingClassification::InvalidNonBijective,
            ranks: [1, 2, 1],
            l_matches: false,
            statuses: ["pass", "fail", "fail"],
            messages: [
                "all targets are reachable",
                "rank(F)=2, expected 3",
                "rank(Mp)=1; L != [0 I]",
            ],
        },
    ];

    for case in cases {
        // When
        let result = validate(&case.source)?;

        // Then
        let [rank_m, rank_f, rank_m_low] = case.ranks;
        let [target_status, bijective_status, natural_status] = case.statuses;
        let [target_message, bijective_message, natural_message] = case.messages;
        assert_eq!(result.classification(), case.classification);
        assert_eq!(
            serde_json::to_value(result.checks()).map_err(|error| error.to_string())?,
            serde_json::json!([
                {
                    "id": "target_reachable",
                    "status": target_status,
                    "observed": {"rank_m": rank_m},
                    "expected": {"rank_m": 1},
                    "message": target_message
                },
                {
                    "id": "bijective",
                    "status": bijective_status,
                    "observed": {"rank_f": rank_f},
                    "expected": {"rank_f": 3},
                    "message": bijective_message
                },
                {
                    "id": "natural_local_address",
                    "status": natural_status,
                    "observed": {
                        "rank_m_low": rank_m_low,
                        "l_matches_preserve_high": case.l_matches
                    },
                    "expected": {
                        "rank_m_low": 1,
                        "l_matches_preserve_high": true
                    },
                    "message": natural_message
                }
            ])
        );
    }
    Ok(())
}

struct GoldenCase {
    source: String,
    classification: MappingClassification,
    ranks: [u8; 3],
    l_matches: bool,
    statuses: [&'static str; 3],
    messages: [&'static str; 3],
}
