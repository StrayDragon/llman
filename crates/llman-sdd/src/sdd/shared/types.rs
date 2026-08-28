//! Item-type vocabulary shared by the `sdd show` / `sdd validate` selectors.
//!
//! The per-command `fmt::Display` labels (i18n) stay with each command;
//! only the type, its canonical spelling, and CLI flag normalization live here.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ItemType {
    Change,
    Spec,
}

impl ItemType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ItemType::Change => "change",
            ItemType::Spec => "spec",
        }
    }
}

/// Parse a `--type` CLI value into an [`ItemType`], `None` if unrecognized.
pub(crate) fn normalize_type(value: Option<&str>) -> Option<ItemType> {
    let value = value?.to_lowercase();
    match value.as_str() {
        "change" => Some(ItemType::Change),
        "spec" => Some(ItemType::Spec),
        _ => None,
    }
}
