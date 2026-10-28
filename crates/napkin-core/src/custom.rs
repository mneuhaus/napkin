//! Units, functions and global variables registered by extensions (Numi's JavaScript API).

use std::collections::HashMap;

use crate::currency;
use crate::units::{BUILTINS, BaseDim, Dimension, Scale, UnitRef};
use crate::{CustomFunction, CustomUnit, RegisterError};

/// What a custom unit is measured in, resolved down to built-in terms.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Base {
    /// A physical dimension; `factor` converts one custom unit to the SI base unit.
    Physical { dim: Dimension, factor: f64 },
    /// A currency; one custom unit is `ratio` of `code`.
    Currency { code: String, ratio: f64 },
}

/// A registered custom unit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnitDef {
    pub(crate) id: String,
    pub(crate) format: String,
    pub(crate) base: Base,
}

/// The extension registry of an engine.
#[derive(Debug, Clone, Default)]
pub(crate) struct Registry {
    pub(crate) units: Vec<UnitDef>,
    unit_phrases: HashMap<String, usize>,
    unit_phrases_folded: HashMap<String, usize>,
    pub(crate) functions: Vec<CustomFunction>,
    function_phrases: HashMap<String, usize>,
    /// Number of words in the longest phrase.
    pub(crate) max_phrase_tokens: usize,
}

/// Collapses runs of whitespace to single spaces.
pub(crate) fn normalize(phrase: &str) -> String {
    phrase.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn phrases(raw: &[String]) -> Vec<String> {
    raw.iter()
        .map(|p| normalize(p))
        .filter(|p| !p.is_empty())
        .collect()
}

/// Rough token count of a phrase as the lexer will split it (words and punctuation).
fn token_count(phrase: &str) -> usize {
    phrase
        .split_whitespace()
        .map(|w| 1 + w.chars().filter(|c| !c.is_alphanumeric()).count())
        .sum()
}

impl Registry {
    /// Validates and adds a custom unit.
    pub(crate) fn add_unit(&mut self, unit: CustomUnit) -> Result<(), RegisterError> {
        let CustomUnit {
            id,
            phrases: raw_phrases,
            base_unit_id,
            ratio,
            format,
        } = unit;
        let id = id.trim().to_owned();
        if id.is_empty() {
            return Err(RegisterError::new("unit id is empty"));
        }
        if self.units.iter().any(|u| u.id == id) {
            return Err(RegisterError::new(format!("unit id `{id}` is already taken")));
        }
        let names = phrases(&raw_phrases);
        if names.is_empty() {
            return Err(RegisterError::new(format!("unit `{id}` has no phrases")));
        }
        if !ratio.is_finite() || ratio <= 0.0 {
            return Err(RegisterError::new(format!(
                "unit `{id}` needs a finite, positive ratio"
            )));
        }
        let base = self.resolve_base(&id, base_unit_id.trim(), ratio)?;
        let index = self.units.len();
        let format = if format.trim().is_empty() {
            id.clone()
        } else {
            format.trim().to_owned()
        };
        self.units.push(UnitDef { id, format, base });
        for phrase in names {
            self.max_phrase_tokens = self.max_phrase_tokens.max(token_count(&phrase));
            self.unit_phrases_folded.insert(phrase.to_lowercase(), index);
            self.unit_phrases.insert(phrase, index);
        }
        Ok(())
    }

    fn resolve_base(&self, id: &str, base_id: &str, ratio: f64) -> Result<Base, RegisterError> {
        let physical = |dim: Dimension, factor: f64| Base::Physical {
            dim,
            factor: factor * ratio,
        };
        let length = |exp| Dimension::of(BaseDim::Length, exp);
        Ok(match base_id {
            "m" => physical(length(1), 1.0),
            "m2" => physical(length(2), 1.0),
            "m3" => physical(length(3), 1.0),
            "second" => physical(Dimension::of(BaseDim::Time, 1), 1.0),
            "percent" => physical(Dimension::default(), 0.01),
            "bit" => physical(Dimension::of(BaseDim::Data, 1), 1.0),
            "byte" => physical(Dimension::of(BaseDim::Data, 1), 8.0),
            "radian" => physical(Dimension::of(BaseDim::Angle, 1), 1.0),
            "gram" => physical(Dimension::of(BaseDim::Mass, 1), 1.0),
            _ if base_id == id => physical(Dimension::of(BaseDim::Custom(self.units.len()), 1), 1.0),
            _ => {
                if let Some(other) = self.units.iter().find(|u| u.id == base_id) {
                    match &other.base {
                        Base::Physical { dim, factor } => physical(dim.clone(), *factor),
                        Base::Currency {
                            code,
                            ratio: base_ratio,
                        } => Base::Currency {
                            code: code.clone(),
                            ratio: base_ratio * ratio,
                        },
                    }
                } else if let Some(code) = currency::canonical(base_id) {
                    Base::Currency {
                        code: code.to_owned(),
                        ratio,
                    }
                } else if let Some(index) = BUILTINS.by_id(base_id) {
                    let def = BUILTINS.def(index);
                    let Scale::Factor(factor) = def.scale else {
                        return Err(RegisterError::new(format!("`{base_id}` cannot be a base unit")));
                    };
                    physical(def.dim.clone(), factor)
                } else {
                    return Err(RegisterError::new(format!(
                        "unknown base unit `{base_id}` for unit `{id}`"
                    )));
                }
            }
        })
    }

    /// Validates and adds a custom function.
    pub(crate) fn add_function(&mut self, function: CustomFunction) -> Result<(), RegisterError> {
        let id = function.id.trim();
        if id.is_empty() {
            return Err(RegisterError::new("function id is empty"));
        }
        if self.functions.iter().any(|f| f.id == id) {
            return Err(RegisterError::new(format!("function id `{id}` is already taken")));
        }
        let names = phrases(&function.phrases);
        if names.is_empty() {
            return Err(RegisterError::new(format!("function `{id}` has no phrases")));
        }
        let index = self.functions.len();
        for phrase in names {
            self.function_phrases.insert(phrase.to_lowercase(), index);
        }
        self.functions.push(function);
        Ok(())
    }

    /// A custom unit named by `phrase` (exact case first, then any case).
    pub(crate) fn unit(&self, phrase: &str) -> Option<usize> {
        self.unit_phrases
            .get(phrase)
            .or_else(|| self.unit_phrases_folded.get(&phrase.to_lowercase()))
            .copied()
    }

    /// A custom unit by id.
    pub(crate) fn unit_by_id(&self, id: &str) -> Option<usize> {
        self.units.iter().position(|u| u.id == id)
    }

    /// A custom function named by `word` (any case).
    pub(crate) fn function(&self, word: &str) -> Option<usize> {
        self.function_phrases.get(&word.to_lowercase()).copied()
    }

    /// Whether `word` names a custom unit or function.
    pub(crate) fn names(&self, word: &str) -> bool {
        self.unit(word).is_some() || self.function(word).is_some()
    }
}

/// A unit reference for a Numi base unit id (`m`, `second`, `m2`, `USD`, ...).
pub(crate) fn numi_base(id: &str) -> Option<(UnitRef, i8)> {
    let builtin = |symbol: &str| BUILTINS.by_id(symbol).map(UnitRef::Builtin);
    Some(match id {
        "m" => (builtin("m")?, 1),
        "m2" => (builtin("m")?, 2),
        "m3" => (builtin("m")?, 3),
        "second" => (builtin("s")?, 1),
        "bit" => (builtin("b")?, 1),
        "byte" => (builtin("B")?, 1),
        "radian" => (builtin("rad")?, 1),
        "gram" => (builtin("g")?, 1),
        "percent" => (UnitRef::Percent, 1),
        _ => return None,
    })
}
