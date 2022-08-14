use crate::parse::{NixGuardedValue, NixOption};
use druid::{Data, Lens};

/// Top-level `Data` instance holding all data of a selected option
#[derive(Clone, Data, Lens)]
pub struct DisplayData {
    documentation: Option<OptionDocumentation>,
    // FIXME: Very hacky, maybe implement PartialEq?
    #[data(ignore)]
    value: Option<NixGuardedValue>,
}

impl std::fmt::Display for DisplayData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.documentation.as_ref(), self.value.as_ref()) {
            (None, _) => write!(f, "No documentation available."),
            (Some(ref d), Some(ref v)) => write!(f, "Value: {}\n\n\n{}", v, d),
            (Some(ref d), None) => d.fmt(f),
        }
    }
}

impl DisplayData {
    pub fn new() -> Self {
        Self {
            documentation: None,
            value: None,
        }
    }

    pub fn new_with(
        documentation: Option<OptionDocumentation>,
        value: Option<NixGuardedValue>,
    ) -> Self {
        Self {
            documentation,
            value,
        }
    }
}

/// `Data` instance holding the static part of the documentation
#[derive(Clone, Data, Debug, Lens)]
pub struct OptionDocumentation {
    pub description: String,
    pub type_name: String,
    #[data(ignore)]
    pub default: Option<NixGuardedValue>,
    #[data(ignore)]
    pub example: Option<NixGuardedValue>,
}

impl From<&NixOption> for OptionDocumentation {
    fn from(opt: &NixOption) -> Self {
        Self {
            description: opt.description.clone(),
            type_name: opt.r#type.to_string(),
            default: opt.default.clone(),
            example: opt.example.clone(),
        }
    }
}

impl std::fmt::Display for OptionDocumentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Description: {}\n\nType: {}",
            self.description, self.type_name
        )?;

        if let Some(ref def) = self.default {
            write!(f, "\n\nDefault: {}", def)?;
        }
        if let Some(ref ex) = self.example {
            write!(f, "\n\nExample: {}", ex)?;
        }

        write!(f, "")
    }
}
