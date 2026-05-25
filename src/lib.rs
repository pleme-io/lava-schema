//! lava-schema — typed interface protocol for lava architectures.
//!
//! Think GraphQL for infrastructure. Each architecture declares an
//! [`Interface`] (typed inputs + typed outputs); other architectures
//! consume it by typed query. Composition typechecks at the
//! evaluator (compile time), never at the cloud API (apply time).
//!
//! ## Strict + loose by design
//!
//! - **Strict fields** carry a [`lava_types::Type`] constraint.
//!   Mismatched / missing values fail the plan.
//! - **Loose fields** use `Type::Any` / `Type::Dynamic` —
//!   intentionally accept any value for extension slots, dynamic
//!   bag-of-tags, runtime-resolved keys, etc.
//!
//! ## Tatara-lisp surface
//!
//! ```lisp
//! (deflava-interface aws-vpc-network
//!   :doc "Standard AWS VPC + IGW + public/private subnets + NAT"
//!   :inputs ((:cidr  :type :cidr-block :default "10.0.0.0/16")
//!            (:azs   :type (:list-of :availability-zone)
//!                    :min-items 1 :default ("us-east-1a" "us-east-1b" "us-east-1c"))
//!            (:env   :type (:enum "prod" "staging" "dev") :default "production")
//!            (:tags  :type :any))                ;; loose
//!   :outputs ((:vpc-id              :type :string :required #t)
//!             (:public-subnet-ids   :type (:list-of :string))
//!             (:private-subnet-ids  :type (:list-of :string))
//!             (:internet-gateway-id :type :string)
//!             (:nat-gateway-ids     :type (:list-of :string))))
//! ```
//!
//! Downstream architectures bind to the named interface:
//!
//! ```lisp
//! (deflava-architecture aws-eks-cluster
//!   :requires ((:network aws-vpc-network))     ;; typed query
//!   :resources
//!   ((aws-eks-cluster "main"
//!      :vpc-config { :subnet-ids (interface-out :network :public-subnet-ids) })))
//! ```
//!
//! At evaluation time the [`InterfaceRegistry`] validates that the
//! `:network` binding satisfies the `aws-vpc-network` interface —
//! mismatched output field, wrong type, missing required field all
//! fail before any cloud API call.

#![allow(clippy::module_name_repetitions)]

use indexmap::IndexMap;
use lava_types::{Type, TypeError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Typed declaration of one architecture's interface — its strict
/// inputs + strict outputs + (optionally) loose extension fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    /// Stable name — `aws-vpc-network`, `aws-eks-cluster`, etc.
    pub name: String,
    pub doc: Option<String>,
    pub inputs: IndexMap<String, Field>,
    pub outputs: IndexMap<String, Field>,
}

impl Interface {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            doc: None,
            inputs: IndexMap::new(),
            outputs: IndexMap::new(),
        }
    }

    /// Validate a candidate input bag against this interface.
    /// Returns errors for: missing required fields, unknown fields
    /// (unless the interface declares an `:any` extension slot),
    /// and per-field typed validation failures.
    pub fn validate_inputs(&self, bag: &IndexMap<String, String>) -> Result<(), Vec<SchemaError>> {
        validate_bag(&self.inputs, bag, "input")
    }

    /// Validate a candidate output bag against this interface.
    /// Same rules as inputs.
    pub fn validate_outputs(&self, bag: &IndexMap<String, String>) -> Result<(), Vec<SchemaError>> {
        validate_bag(&self.outputs, bag, "output")
    }
}

/// One field in an interface — strict type + flags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub r#type: Type,
    pub required: bool,
    pub default: Option<String>,
    pub doc: Option<String>,
    /// Loose field — `:any` / `:dynamic` types should set this true
    /// so the registry's "unknown field" check doesn't reject extension
    /// slots.
    pub loose: bool,
}

impl Field {
    #[must_use]
    pub fn strict(t: Type) -> Self {
        let loose = matches!(t, Type::Any | Type::Dynamic);
        Self {
            r#type: t,
            required: true,
            default: None,
            doc: None,
            loose,
        }
    }

    #[must_use]
    pub fn optional(t: Type) -> Self {
        let loose = matches!(t, Type::Any | Type::Dynamic);
        Self {
            r#type: t,
            required: false,
            default: None,
            doc: None,
            loose,
        }
    }

    #[must_use]
    pub fn with_default(mut self, d: impl Into<String>) -> Self {
        self.default = Some(d.into());
        self.required = false;
        self
    }

    #[must_use]
    pub fn with_doc(mut self, d: impl Into<String>) -> Self {
        self.doc = Some(d.into());
        self
    }
}

fn validate_bag(
    fields: &IndexMap<String, Field>,
    bag: &IndexMap<String, String>,
    label: &'static str,
) -> Result<(), Vec<SchemaError>> {
    let mut errors = Vec::new();
    // Required fields must be present.
    for (name, field) in fields {
        if field.required && !bag.contains_key(name) && field.default.is_none() {
            errors.push(SchemaError::MissingRequired {
                kind: label,
                name: name.clone(),
            });
        }
    }
    // Unknown fields fail UNLESS the interface has at least one loose
    // field (an `:any` extension slot signals open-set semantics).
    let allow_unknown = fields.values().any(|f| f.loose);
    for (name, value) in bag {
        match fields.get(name) {
            None => {
                if !allow_unknown {
                    errors.push(SchemaError::UnknownField {
                        kind: label,
                        name: name.clone(),
                    });
                }
            }
            Some(field) => {
                // For ListOf fields, the bag carries a comma-joined
                // string projection (lava-runtime + lava-architectures
                // both project list bindings that way). Split + run
                // validate_list so the typed inner check fires.
                let result = match &field.r#type {
                    Type::ListOf { .. } => {
                        let parts: Vec<&str> = if value.is_empty() {
                            Vec::new()
                        } else {
                            value.split(',').collect()
                        };
                        field.r#type.validate_list(&parts)
                    }
                    _ => field.r#type.validate(value),
                };
                if let Err(e) = result {
                    errors.push(SchemaError::TypeMismatch {
                        kind: label,
                        name: name.clone(),
                        cause: e,
                    });
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Registry of every known architecture's typed interface. Composition
/// queries route through here.
#[derive(Debug, Default)]
pub struct InterfaceRegistry {
    by_name: IndexMap<String, Interface>,
}

impl InterfaceRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, iface: Interface) {
        self.by_name.insert(iface.name.clone(), iface);
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Interface> {
        self.by_name.get(name)
    }

    #[must_use]
    pub fn names(&self) -> Vec<&String> {
        self.by_name.keys().collect()
    }

    /// True when `provider_iface` satisfies every output the
    /// `consumer_iface` requires under `:requires` clause. Drives the
    /// typed cross-architecture wiring at composition time.
    #[must_use]
    pub fn provides(&self, provider: &Interface, required_outputs: &[String]) -> Vec<String> {
        let mut missing = Vec::new();
        for needed in required_outputs {
            if !provider.outputs.contains_key(needed) {
                missing.push(needed.clone());
            }
        }
        missing
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum SchemaError {
    #[error("missing required {kind} `{name}`")]
    MissingRequired { kind: &'static str, name: String },
    #[error("unknown {kind} `{name}` (no loose-fields slot declared)")]
    UnknownField { kind: &'static str, name: String },
    #[error("type error on {kind} `{name}`: {cause}")]
    TypeMismatch {
        kind: &'static str,
        name: String,
        cause: TypeError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aws_vpc_network_interface() -> Interface {
        let mut iface = Interface::new("aws-vpc-network");
        iface.doc = Some("Standard AWS VPC + IGW + subnets + NAT".to_string());
        iface.inputs.insert(
            "cidr".to_string(),
            Field::optional(Type::CidrBlock).with_default("10.0.0.0/16"),
        );
        iface.inputs.insert(
            "azs".to_string(),
            Field::optional(Type::ListOf {
                inner: Box::new(Type::AvailabilityZone),
                min_items: Some(1),
                max_items: Some(6),
            }),
        );
        iface.inputs.insert(
            "env".to_string(),
            Field::optional(Type::Enum {
                values: vec!["prod".into(), "staging".into(), "dev".into()],
            })
            .with_default("prod"),
        );
        iface.outputs.insert(
            "vpc-id".to_string(),
            Field::strict(Type::String).with_doc("VPC ID reference"),
        );
        iface.outputs.insert(
            "public-subnet-ids".to_string(),
            Field::strict(Type::ListOf {
                inner: Box::new(Type::String),
                min_items: None,
                max_items: None,
            }),
        );
        iface.outputs.insert(
            "internet-gateway-id".to_string(),
            Field::optional(Type::String),
        );
        iface
    }

    #[test]
    fn validate_inputs_passes_on_valid_bag() {
        let iface = aws_vpc_network_interface();
        let mut bag = IndexMap::new();
        bag.insert("cidr".to_string(), "10.0.0.0/16".to_string());
        bag.insert("env".to_string(), "prod".to_string());
        assert!(iface.validate_inputs(&bag).is_ok());
    }

    #[test]
    fn validate_inputs_rejects_invalid_cidr() {
        let iface = aws_vpc_network_interface();
        let mut bag = IndexMap::new();
        bag.insert("cidr".to_string(), "not-a-cidr".to_string());
        let errs = iface.validate_inputs(&bag).unwrap_err();
        assert!(matches!(errs[0], SchemaError::TypeMismatch { .. }));
    }

    #[test]
    fn validate_inputs_rejects_unknown_field_when_no_loose_slot() {
        let iface = aws_vpc_network_interface();
        let mut bag = IndexMap::new();
        bag.insert("unknown".to_string(), "x".to_string());
        let errs = iface.validate_inputs(&bag).unwrap_err();
        assert!(matches!(errs[0], SchemaError::UnknownField { .. }));
    }

    #[test]
    fn validate_inputs_accepts_unknown_field_when_loose_slot_present() {
        let mut iface = aws_vpc_network_interface();
        iface
            .inputs
            .insert("tags".to_string(), Field::optional(Type::Any));
        let mut bag = IndexMap::new();
        bag.insert("cidr".to_string(), "10.0.0.0/16".to_string());
        bag.insert("anything-goes".to_string(), "even-this".to_string());
        // Loose slot makes the interface open-set; unknown field passes.
        assert!(iface.validate_inputs(&bag).is_ok());
    }

    #[test]
    fn validate_inputs_rejects_enum_miss() {
        let iface = aws_vpc_network_interface();
        let mut bag = IndexMap::new();
        bag.insert("env".to_string(), "preview".to_string());
        let errs = iface.validate_inputs(&bag).unwrap_err();
        assert!(matches!(errs[0], SchemaError::TypeMismatch { .. }));
    }

    #[test]
    fn registry_provides_check_finds_missing_outputs() {
        let iface = aws_vpc_network_interface();
        let registry = InterfaceRegistry::new();
        let needed = vec![
            "vpc-id".to_string(),
            "public-subnet-ids".to_string(),
            "kubeconfig".to_string(),
        ];
        let missing = registry.provides(&iface, &needed);
        assert_eq!(missing, vec!["kubeconfig".to_string()]);
    }

    #[test]
    fn interface_round_trips_through_serde() {
        // .caixa.lisp ingests typed interfaces via this exact path.
        let iface = aws_vpc_network_interface();
        let json = serde_json::to_string(&iface).unwrap();
        let parsed: Interface = serde_json::from_str(&json).unwrap();
        assert_eq!(iface, parsed);
    }
}

