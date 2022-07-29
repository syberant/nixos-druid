use druid::im;
use druid::{Data, Lens};
use nix_druid::parse::{NixGuardedValue, NixTypeValue, NixValue};

#[derive(Clone, Data, Debug, Lens)]
pub struct OptionDocumentation {
    pub description: String,
    // pub default: Option<NixGuardedValue>,
    // pub example: Option<NixGuardedValue>,
}

impl From<&nix_druid::parse::NixOption> for OptionDocumentation {
    fn from(opt: &nix_druid::parse::NixOption) -> Self {
        Self {
            description: opt.description.clone(),
        }
    }
}

impl std::fmt::Display for OptionDocumentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Description: {}", self.description)
    }
}

#[derive(Clone, Data, Debug, Lens)]
pub struct OptionNode {
    pub name: String,
    pub documentation: Option<OptionDocumentation>,
    pub option_type: Option<OptionType>,
    pub value: Option<NixGuardedValue>,
    pub children: im::Vector<OptionNode>,
    pub expanded: bool,
}

impl OptionNode {
    fn new_option(mut name: String, option_type: OptionType, doc: OptionDocumentation) -> Self {
        use im::Vector;
        use OptionType::*;

        // I want to stay on stable rust,
        // with unstable one could use `box_patterns` here
        let children = match option_type {
            AttrsOf(ref t) => {
                if let Submodule(ref sub) = **t {
                    name.push_str(".<name>");

                    sub.to_owned()
                } else {
                    Vector::new()
                }
            }
            ListOf(ref t) => {
                if let Submodule(ref sub) = **t {
                    name.push_str(".*");

                    sub.to_owned()
                } else {
                    Vector::new()
                }
            }
            _ => Vector::new(),
        };

        Self {
            name,
            documentation: Some(doc),
            option_type: Some(option_type),
            value: None,
            children,
            expanded: false,
        }
    }

    fn new_set(name: String, mut children: im::Vector<OptionNode>) -> Self {
        children.sort_by(|left, right| left.name.cmp(&right.name));

        Self {
            name,
            documentation: None,
            option_type: None,
            value: None,
            children,
            expanded: false,
        }
    }

    pub fn new(name: String, val: NixValue) -> Self {
        match val {
            NixValue::Option(opt) => {
                let doc = (&opt).into();
                Self::new_option(name, opt.r#type.into(), doc)
            }
            NixValue::Set(set) => {
                let children = set
                    .into_iter()
                    .map(|(k, v)| OptionNode::new(k, *v))
                    .collect();
                Self::new_set(name, children)
            }
        }
    }

    pub fn get_documentation(&self) -> Option<String> {
        self.option_type
            .as_ref()
            .map(|_| "Documentation not yet handled".to_string())
    }
}

#[derive(Clone, Debug)]
pub enum OptionType {
    // Collection types
    AttrsOf(Box<OptionType>),
    ListOf(Box<OptionType>),
    NullOr(Box<OptionType>),
    Either(Box<OptionType>, Box<OptionType>),

    // Leaf/simple types
    Enum(im::Vector<String>),
    Path,
    Package,
    String,
    Float,
    Bool,
    /// Int with optional min/max bounds
    Int(Option<i64>, Option<i64>),

    // Miscellaneous types
    Unknown(String),
    Submodule(im::Vector<OptionNode>),
}

impl Data for OptionType {
    fn same(&self, other: &Self) -> bool {
        use OptionType::*;

        match (self, other) {
            (AttrsOf(l), AttrsOf(r)) => l.same(r),
            (ListOf(l), ListOf(r)) => l.same(r),
            (NullOr(l), NullOr(r)) => l.same(r),
            (Either(l1, l2), Either(r1, r2)) => l1.same(r1) && l2.same(r2),

            (Enum(l), Enum(r)) => l.same(r),
            (Path, Path) => true,
            (Package, Package) => true,
            (String, String) => true,
            (Float, Float) => true,
            (Bool, Bool) => true,
            (Int(l1, l2), Int(r1, r2)) => l1.same(r1) && l2.same(r2),

            (Unknown(l), Unknown(r)) => l.same(r),
            (Submodule(l), Submodule(r)) => l.same(r),

            _ => false,
        }
    }
}

impl From<NixTypeValue> for OptionType {
    fn from(raw_type: NixTypeValue) -> Self {
        use NixTypeValue::*;
        use OptionType::*;

        match raw_type {
            Type(mut t) => {
                // Use `remove` here to avoid borrow and take ownership of value
                if let Some(elem) = t.nestedTypes.remove("elemType") {
                    match t.name.as_ref() {
                        "nullOr" => NullOr(Box::new(elem.into())),
                        "listOf" => ListOf(Box::new(elem.into())),
                        "attrsOf" | "lazyAttrsOf" => AttrsOf(Box::new(elem.into())),
                        _ => Unknown(Type(t).to_string()),
                    }
                } else if let (Some(left), Some(right)) =
                    (t.nestedTypes.remove("left"), t.nestedTypes.remove("right"))
                {
                    match t.name.as_ref() {
                        "either" => Either(Box::new(left.into()), Box::new(right.into())),
                        _ => Unknown(Type(t).to_string()),
                    }
                } else {
                    match t.name.as_ref() {
                        "str" | "string" => String,
                        // lib.types.{commas, lines} => separatedString
                        "separatedString" => String,

                        "int" => Int(None, None),
                        "positiveInt" => Int(Some(1), None),
                        "unsignedInt" => Int(Some(0), None),
                        "unsignedInt8" => Int(Some(0), Some((1 >> 8) - 1)),
                        "unsignedInt16" => Int(Some(0), Some((1 >> 16) - 1)),
                        "unsignedInt32" => Int(Some(0), Some((1 >> 32) - 1)),
                        "signedInt8" => Int(Some(-(1 >> 7)), Some((1 >> 7) - 1)),
                        "signedInt16" => Int(Some(-(1 >> 15)), Some((1 >> 15) - 1)),
                        "signedInt32" => Int(Some(-(1 >> 31)), Some((1 >> 31) - 1)),

                        "float" => Float,
                        "bool" => Bool,
                        "path" => Path,
                        "package" => Package,

                        _ => Unknown(Type(t).to_string()),
                    }
                }
            }
            NixTypeValue::Submodule(set) => {
                let children = set
                    .options
                    .into_iter()
                    .map(|(k, v)| OptionNode::new(k, *v))
                    .collect();
                OptionType::Submodule(children)
            }
            InfiniteRecursion(r) => Unknown(InfiniteRecursion(r).to_string()),
        }
    }
}
