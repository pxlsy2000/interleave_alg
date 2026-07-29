use std::{
    cell::Cell,
    io::{self, Read},
    rc::Rc,
};

use interleave::input::{
    InputLoadError, SpannedMappingEntry, SpannedYamlDocument, SpannedYamlKind, SpannedYamlScalar,
    load_yaml_bytes,
};

type TestResult<T = ()> = Result<T, String>;

fn fixture(name: &str) -> TestResult<&'static [u8]> {
    match name {
        "valid" => Ok(include_bytes!("../fixtures/yaml/valid_comments.yaml")),
        "nested_flow" => Ok(include_bytes!("../fixtures/yaml/nested_flow.yaml")),
        "alias" => Ok(include_bytes!("../fixtures/yaml/alias.yaml")),
        "merge" => Ok(include_bytes!("../fixtures/yaml/merge.yaml")),
        "tag" => Ok(include_bytes!("../fixtures/yaml/explicit_tag.yaml")),
        "non_string" => Ok(include_bytes!("../fixtures/yaml/non_string_key.yaml")),
        "two_documents" => Ok(include_bytes!("../fixtures/yaml/two_documents.yaml")),
        "flow_root" => Ok(include_bytes!("../fixtures/yaml/flow_root.yaml")),
        unknown => Err(format!("unknown fixture {unknown}")),
    }
}

fn yaml_error(bytes: &[u8]) -> TestResult<InputLoadError> {
    match load_yaml_bytes(bytes) {
        Ok(_) => Err("fixture was unexpectedly accepted".to_owned()),
        Err(error) => Ok(error),
    }
}

fn mapping(document: &SpannedYamlDocument) -> TestResult<&[SpannedMappingEntry]> {
    match document.root().kind() {
        SpannedYamlKind::Mapping(entries) => Ok(entries),
        SpannedYamlKind::Scalar(_) | SpannedYamlKind::Sequence(_) => {
            Err("expected mapping root".to_owned())
        }
    }
}

fn scalar_at(
    entries: &[SpannedMappingEntry],
    index: usize,
) -> TestResult<&SpannedYamlScalar> {
    entries
        .get(index)
        .and_then(|entry| entry.value().as_scalar())
        .ok_or_else(|| format!("expected scalar entry at {index}"))
}

#[derive(Debug)]
struct ObservedReader {
    remaining: usize,
    consumed: Rc<Cell<usize>>,
}

impl ObservedReader {
    fn new(remaining: usize) -> (Self, Rc<Cell<usize>>) {
        let consumed = Rc::new(Cell::new(0));
        (
            Self {
                remaining,
                consumed: Rc::clone(&consumed),
            },
            consumed,
        )
    }
}

impl Read for ObservedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.remaining.min(buffer.len()).min(3);
        let Some(received) = buffer.get_mut(..count) else {
            return Err(io::Error::other("invalid test reader request"));
        };
        received.fill(b'x');
        self.remaining -= count;
        self.consumed.set(self.consumed.get().saturating_add(count));
        Ok(count)
    }
}
