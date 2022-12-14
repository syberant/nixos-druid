use druid::im;
use druid::{Data, Lens};
use nixos_druid::data::DisplayData;
use nixos_druid::data::OptionDocumentation;
use nixos_druid::parse::{NixGuardedValue, NixTypeValue, NixValue};
use nixos_druid::tree_node::TreeOptionNode;

#[derive(Clone, Data, Debug, Lens)]
pub struct OptionNode {
    pub name: String,
    pub documentation: Option<OptionDocumentation>,
    pub option_type: Option<OptionType>,
    // TODO: figure out a way to deal with this,
    // other characteristics should be enough to make
    // `Data` trait still work though
    #[data(ignore)]
    pub value: Option<NixGuardedValue>,
    pub children: im::Vector<OptionNode>,
    /// An extra child for options where children are of type `Submodule`.
    /// This extra child gives a way to view documentation when there are no
    /// real children.
    #[data(ignore)]
    pub extra_child: Option<Box<OptionNode>>,
    pub expanded: bool,
}

impl OptionNode {
    fn new_option(name: String, option_type: OptionType, doc: OptionDocumentation) -> Self {
        use im::Vector;
        use OptionType::*;

        // I want to stay on stable rust,
        // with unstable one could use `box_patterns` here
        let children = match option_type {
            NullOr(ref t) => {
                if let Submodule(ref sub) = **t {
                    sub.to_owned()
                } else {
                    Vector::new()
                }
            }
            Submodule(ref sub) => sub.to_owned(),
            _ => Vector::new(),
        };

        let extra_child = match option_type {
            AttrsOf(ref t) => {
                if let Submodule(ref sub) = **t {
                    Some(Box::new(OptionNode::new_set(
                        "<name>".to_string(),
                        sub.to_owned(),
                    )))
                } else {
                    None
                }
            }
            ListOf(ref t) => {
                if let Submodule(ref sub) = **t {
                    Some(Box::new(OptionNode::new_set(
                        "*".to_string(),
                        sub.to_owned(),
                    )))
                } else {
                    None
                }
            }
            _ => None,
        };

        Self {
            name,
            documentation: Some(doc),
            option_type: Some(option_type),
            value: None,
            children,
            extra_child,
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
            extra_child: None,
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

    fn new_from_submodule(
        name: String,
        sub: im::Vector<OptionNode>,
        cfg: Option<NixGuardedValue>,
    ) -> Self {
        // TODO: Add documentation + value?
        let mut node = Self::new_set(name, sub);
        node.add_config(cfg);
        node
    }

    pub fn add_config(&mut self, mut cfg: Option<NixGuardedValue>) {
        use self::OptionType::*;
        use NixGuardedValue::*;

        if let Some(ref t) = self.option_type {
            match t {
                AttrsOf(ref nt) => {
                    if let Submodule(ref sub) = **nt {
                        if let Some(Attrs(hashmap)) = cfg {
                            self.children = hashmap
                                .into_iter()
                                .map(|(name, child_cfg)| {
                                    OptionNode::new_from_submodule(
                                        name,
                                        sub.to_owned(),
                                        Some(*child_cfg),
                                    )
                                })
                                .collect();
                            self.children
                                .sort_by(|left, right| left.name.cmp(&right.name));
                        }
                    } else {
                        self.value = cfg;
                    }
                }
                ListOf(ref nt) => {
                    if let Submodule(ref sub) = **nt {
                        if let Some(List(list)) = cfg {
                            self.children = list
                                .into_iter()
                                .enumerate()
                                .map(|(counter, child_cfg)| {
                                    OptionNode::new_from_submodule(
                                        format!("Element {}", counter),
                                        sub.to_owned(),
                                        Some(*child_cfg),
                                    )
                                })
                                .collect();
                            self.children
                                .sort_by(|left, right| left.name.cmp(&right.name));
                        }
                    } else {
                        self.value = cfg;
                    }
                }
                Submodule(_) => {
                    // A submodule is also kind of a set
                    for ref mut c in self.children.iter_mut() {
                        let child_cfg = cfg
                            .as_mut()
                            .map(|val| {
                                if let NixGuardedValue::Attrs(ref mut attrs) = val {
                                    attrs.remove(&c.name)
                                } else {
                                    None
                                }
                            })
                            .flatten()
                            .map(|val| *val);
                        c.add_config(child_cfg);
                    }
                }

                _ => self.value = cfg,
            }
        } else {
            // This is a set
            for ref mut c in self.children.iter_mut() {
                let child_cfg = cfg
                    .as_mut()
                    .map(|val| {
                        if let NixGuardedValue::Attrs(ref mut attrs) = val {
                            attrs.remove(&c.name)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .map(|val| *val);
                c.add_config(child_cfg);
            }
        }
    }
}

impl TreeOptionNode for OptionNode {
    // TODO: Nice icons
    fn get_icon(&self) -> String {
        if let Some(ref t) = self.option_type {
            if t.has_nested_submodule() {
                if self.expanded {
                    "????"
                } else {
                    "????"
                }
            } else {
                match self.name.as_ref() {
                    "enable" => "???",
                    _ => "??????",
                }
            }
        } else {
            if self.expanded {
                "????"
            } else {
                "????"
            }
        }
        .to_owned()
    }

    fn focused_display_data(&self) -> DisplayData {
        DisplayData::new_with(self.documentation.clone(), self.value.clone())
    }

    fn is_expanded(&self) -> bool {
        self.expanded
    }

    fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
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

impl OptionType {
    pub fn has_nested_submodule(&self) -> bool {
        use OptionType::*;

        if let Submodule(_) = self {
            true
        } else {
            self.get_name_extension().is_some()
        }
    }

    pub fn get_name_extension(&self) -> Option<&str> {
        use OptionType::*;

        match self {
            AttrsOf(ref t) => {
                if let Submodule(_) = **t {
                    Some("<name>")
                } else {
                    None
                }
            }
            ListOf(ref t) => {
                if let Submodule(_) = **t {
                    Some("*")
                } else {
                    None
                }
            }
            _ => None,
        }
    }
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

                        // TODO: Parse enum
                        // "enum" => Enum(t.functorPayload),
                        _ => Unknown(Type(t).to_string()),
                    }
                }
            }
            NixTypeValue::Submodule(set) => {
                let mut children: im::Vector<OptionNode> = set
                    .options
                    .into_iter()
                    .map(|(k, v)| OptionNode::new(k, *v))
                    .collect();
                children.sort_by(|left, right| left.name.cmp(&right.name));
                OptionType::Submodule(children)
            }
            InfiniteRecursion(r) => Unknown(InfiniteRecursion(r).to_string()),
        }
    }
}
