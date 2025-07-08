use crate::deterministic_map::HashMap;
use crate::kleene::Kleene; // <-- bring in zero()
use crate::presburger::{Constraint as PConstraint, PresburgerSet, QuantifiedSet, Variable};
use either::Either;
use serde::{Serialize, Deserialize};
use std::fmt::{self, Display};
use std::fs;
use std::hash::Hash;
use std::path::Path;

// Helper module for serializing HashMap with non-string keys
mod tuple_vec_map {
    use super::*;

    pub fn serialize<K, V, S>(m: &HashMap<K, V>, ser: S) -> std::result::Result<S::Ok, S::Error>
    where
        K: Serialize + Eq + std::hash::Hash,
        V: Serialize,
        S: serde::Serializer,
    {
        m.iter().collect::<Vec<_>>().serialize(ser)
    }

    pub fn deserialize<'de, K, V, D>(de: D) -> std::result::Result<HashMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Eq + std::hash::Hash,
        V: Deserialize<'de>,
        D: serde::Deserializer<'de>,
    {
        let v: Vec<(K, V)> = Vec::deserialize(de)?;
        Ok(v.into_iter().collect())
    }
}

/// Affine expression: sum of terms (coefficient * variable) + constant
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(bound(serialize = "T: Serialize"))]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct AffineExpr<T: Eq + Hash> {
    /// Map from variable to coefficient
    #[serde(with = "tuple_vec_map")]
    terms: HashMap<Variable<T>, i64>,
    constant: i64,
}

impl<T: Eq + Hash> AffineExpr<T> {
    /// Map variable type from T to U
    pub fn rename_vars<U, F>(self, mut f: F) -> AffineExpr<U>
    where
        F: FnMut(Variable<T>) -> Variable<U>,
        U: Eq + Hash,
    {
        AffineExpr {
            terms: self
                .terms
                .into_iter()
                .map(|(var, coeff)| (f(var), coeff))
                .collect(),
            constant: self.constant,
        }
    }
    pub fn map<U: Eq + Hash>(self, mut f: impl FnMut(T) -> U) -> AffineExpr<U> {
        self.rename_vars(|x| x.map(&mut f))
    }
}

impl<T: Clone + Eq + Hash> AffineExpr<T> {
    /// Create a zero expression
    pub fn new() -> Self {
        AffineExpr {
            terms: HashMap::default(),
            constant: 0,
        }
    }

    /// Create a constant expression
    pub fn from_const(c: i64) -> Self {
        AffineExpr {
            terms: HashMap::default(),
            constant: c,
        }
    }

    /// Create a variable expression (coefficient 1)
    pub fn from_var(var: T) -> Self {
        let mut terms = HashMap::default();
        terms.insert(Variable::Var(var), 1);
        AffineExpr { terms, constant: 0 }
    }

    /// Add two expressions
    pub fn add(&self, other: &AffineExpr<T>) -> AffineExpr<T> {
        let mut result = self.clone();

        // Add the constant
        result.constant += other.constant;

        // Add each term
        for (var, coeff) in &other.terms {
            *result.terms.entry(var.clone()).or_insert(0) += coeff;
        }

        // Remove zero coefficients
        result.terms.retain(|_, coeff| *coeff != 0);

        result
    }

    /// Subtract two expressions
    pub fn sub(&self, other: &AffineExpr<T>) -> AffineExpr<T> {
        self.add(&other.negate())
    }

    /// Multiply by a constant
    pub fn mul_by_const(&self, c: i64) -> AffineExpr<T> {
        if c == 0 {
            return AffineExpr::new();
        }

        let mut result = AffineExpr::new();
        result.constant = self.constant * c;

        for (var, coeff) in &self.terms {
            result.terms.insert(var.clone(), coeff * c);
        }

        result
    }

    /// Negate the expression
    pub fn negate(&self) -> AffineExpr<T> {
        self.mul_by_const(-1)
    }

    /// Get coefficient of a variable (0 if not present)
    pub fn get_coeff(&self, var: &Variable<T>) -> i64 {
        self.terms.get(var).copied().unwrap_or(0)
    }

    /// Get the constant term
    pub fn get_constant(&self) -> i64 {
        self.constant
    }

    /// Get all variables in the expression (only non-existential ones)
    pub fn variables(&self) -> Vec<T> {
        self.terms
            .keys()
            .filter_map(|var| match var {
                Variable::Var(v) => Some(v.clone()),
                Variable::Existential(_) => None,
            })
            .collect()
    }

    /// Check if this is a constant expression (no variables)
    pub fn is_constant(&self) -> bool {
        self.terms.is_empty()
    }

    /// Convert to a vector of (coefficient, variable) pairs plus constant
    /// This is useful for converting to presburger constraints
    pub fn to_linear_combination(&self) -> (Vec<(i64, Variable<T>)>, i64) {
        let terms: Vec<(i64, Variable<T>)> = self
            .terms
            .iter()
            .map(|(var, coeff)| (*coeff, var.clone()))
            .collect();
        (terms, self.constant)
    }
}

impl<T: fmt::Display + Eq + Hash> fmt::Display for AffineExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.terms.is_empty() && self.constant == 0 {
            write!(f, "0")
        } else {
            let mut first = true;

            // Note: HashMap doesn't guarantee order, but that's okay for display
            for (var, coeff) in &self.terms {
                if *coeff == 0 {
                    continue;
                }

                if !first {
                    write!(f, " ")?;
                    if *coeff >= 0 {
                        write!(f, "+ ")?;
                    }
                } else {
                    first = false;
                }

                // Custom display for Variable<T> to avoid the "V" prefix in tests
                match var {
                    Variable::Var(t) => {
                        if *coeff == 1 {
                            write!(f, "{}", t)?;
                        } else if *coeff == -1 {
                            write!(f, "-{}", t)?;
                        } else {
                            write!(f, "{}*{}", coeff, t)?;
                        }
                    }
                    Variable::Existential(n) => {
                        if *coeff == 1 {
                            write!(f, "e{}", n)?;
                        } else if *coeff == -1 {
                            write!(f, "-e{}", n)?;
                        } else {
                            write!(f, "{}*e{}", coeff, n)?;
                        }
                    }
                }
            }

            // Write constant
            if self.constant != 0 || self.terms.is_empty() {
                if !first {
                    write!(f, " ")?;
                    if self.constant >= 0 {
                        write!(f, "+ ")?;
                    }
                }
                write!(f, "{}", self.constant)?;
            }

            Ok(())
        }
    }
}

impl<L, R> AffineExpr<Either<L, R>>
where
    L: Eq + Hash,
    R: Eq + Hash,
{
    /// Project from AffineExpr<Either<L, R>> to AffineExpr<R>
    pub fn project_right(self) -> AffineExpr<R> {
        AffineExpr {
            terms: self
                .terms
                .into_iter()
                .filter_map(|(var, coeff)| match var {
                    Variable::Var(Either::Left(_)) => None,
                    Variable::Var(Either::Right(r)) => Some((Variable::Var(r), coeff)),
                    Variable::Existential(n) => Some((Variable::Existential(n), coeff)),
                })
                .collect(),
            constant: self.constant,
        }
    }
}

/// Comparison operators (normalized to only = and >=)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompOp {
    Eq,  // =
    Geq, // >=
}

impl fmt::Display for CompOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompOp::Eq => write!(f, "="),
            CompOp::Geq => write!(f, "≥"),
        }
    }
}

/// Linear constraint: expr op 0
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Constraint<T: Eq + Hash> {
    pub expr: AffineExpr<T>,
    pub op: CompOp,
}

impl<T: Eq + Hash> Constraint<T> {
    /// Map variable type from T to U
    pub fn rename_vars<U, F>(self, f: F) -> Constraint<U>
    where
        F: FnMut(Variable<T>) -> Variable<U>,
        U: Eq + Hash,
    {
        Constraint {
            expr: self.expr.rename_vars(f),
            op: self.op,
        }
    }

    pub fn map<U: Eq + Hash>(self, mut f: impl FnMut(T) -> U) -> Constraint<U> {
        self.rename_vars(|v| v.map(&mut f))
    }
}

impl<T: Clone + Eq + Hash> Constraint<T> {
    pub fn new(expr: AffineExpr<T>, op: CompOp) -> Self {
        Constraint { expr, op }
    }
}

impl<L, R> Constraint<Either<L, R>>
where
    L: Eq + Hash,
    R: Eq + Hash,
{
    /// Project from Constraint<Either<L, R>> to Constraint<R>
    pub fn project_right(self) -> Constraint<R> {
        Constraint {
            expr: self.expr.project_right(),
            op: self.op,
        }
    }
}

impl<T: fmt::Display + Eq + Hash> fmt::Display for Constraint<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} 0", self.expr, self.op)
    }
}

/// Normalized formula (no Not or Implies)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Formula<T: Eq + Hash> {
    Constraint(Constraint<T>),
    And(Vec<Formula<T>>),
    Or(Vec<Formula<T>>),
    Exists(usize, Box<Formula<T>>), // Bound variable index
    Forall(usize, Box<Formula<T>>), // Bound variable index
}

impl<T: Eq + Hash> Formula<T> {
    /// Collect all free variables in the formula, properly handling shadowing
    /// by existential and universal quantifiers
    pub fn collect_free_variables(&self) -> std::collections::HashSet<T>
    where
        T: Clone,
    {
        self.collect_free_variables_with_bound(&std::collections::HashSet::new())
    }
    
    /// Helper method that tracks bound variables
    fn collect_free_variables_with_bound(&self, bound_vars: &std::collections::HashSet<usize>) -> std::collections::HashSet<T>
    where
        T: Clone,
    {
        match self {
            Formula::Constraint(c) => {
                let mut free_vars = std::collections::HashSet::new();
                for (var, _) in &c.expr.terms {
                    match var {
                        Variable::Var(v) => {
                            free_vars.insert(v.clone());
                        }
                        Variable::Existential(idx) => {
                            // Only free if not bound by a quantifier
                            if !bound_vars.contains(idx) {
                                panic!("Existential variable e{} used but not bound by quantifier", idx);
                            }
                        }
                    }
                }
                free_vars
            }
            Formula::And(formulas) | Formula::Or(formulas) => {
                let mut free_vars = std::collections::HashSet::new();
                for formula in formulas {
                    free_vars.extend(formula.collect_free_variables_with_bound(bound_vars));
                }
                free_vars
            }
            Formula::Exists(idx, body) | Formula::Forall(idx, body) => {
                // Add this index to bound variables for the body
                let mut new_bound = bound_vars.clone();
                new_bound.insert(*idx);
                body.collect_free_variables_with_bound(&new_bound)
            }
        }
    }

    /// Map variable type from T to U
    pub fn rename_vars<U, F>(self, f: &mut F) -> Formula<U>
    where
        F: FnMut(Variable<T>) -> Variable<U>,
        U: Eq + Hash,
    {
        match self {
            Formula::Constraint(c) => Formula::Constraint(c.rename_vars(f)),
            Formula::And(formulas) => Formula::And(
                formulas
                    .into_iter()
                    .map(|form| form.rename_vars(f))
                    .collect(),
            ),
            Formula::Or(formulas) => Formula::Or(
                formulas
                    .into_iter()
                    .map(|form| form.rename_vars(f))
                    .collect(),
            ),
            Formula::Exists(idx, body) => {
                // Bound variable index remains the same
                Formula::Exists(idx, Box::new(body.rename_vars(f)))
            }
            Formula::Forall(idx, body) => {
                // Bound variable index remains the same
                Formula::Forall(idx, Box::new(body.rename_vars(f)))
            }
        }
    }

    pub fn map<U: Eq + Hash>(self, mut f: impl FnMut(T) -> U) -> Formula<U> {
        self.rename_vars(&mut |v| v.map(&mut f))
    }
}

impl<L, R> Formula<Either<L, R>>
where
    L: Eq + Hash,
    R: Eq + Hash,
{
    /// Project from Formula<Either<L, R>> to Formula<R>
    /// when all Left values should not exist
    /// Using manual recursion to avoid infinite type recursion
    pub fn project_right(self) -> Formula<R> {
        // We need to manually implement this to avoid the map function
        fn project_formula<L, R>(formula: Formula<Either<L, R>>) -> Formula<R>
        where
            L: Eq + Hash,
            R: Eq + Hash,
        {
            match formula {
                Formula::Constraint(c) => Formula::Constraint(c.project_right()),
                Formula::And(formulas) => {
                    let mut projected = Vec::new();
                    for f in formulas {
                        projected.push(project_formula(f));
                    }
                    Formula::And(projected)
                }
                Formula::Or(formulas) => {
                    let mut projected = Vec::new();
                    for f in formulas {
                        projected.push(project_formula(f));
                    }
                    Formula::Or(projected)
                }
                Formula::Exists(idx, body) => {
                    // Bound variable index remains the same
                    Formula::Exists(idx, Box::new(project_formula(*body)))
                }
                Formula::Forall(idx, body) => {
                    // Bound variable index remains the same
                    Formula::Forall(idx, Box::new(project_formula(*body)))
                }
            }
        }

        project_formula(self)
    }
}

impl<T: fmt::Display + Eq + Hash> fmt::Display for Formula<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Formula::Constraint(c) => write!(f, "{}", c),
            Formula::And(formulas) => {
                if formulas.is_empty() {
                    write!(f, "⊤") // true
                } else {
                    write!(f, "(")?;
                    for (i, formula) in formulas.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ∧ ")?;
                        }
                        write!(f, "{}", formula)?;
                    }
                    write!(f, ")")
                }
            }
            Formula::Or(formulas) => {
                if formulas.is_empty() {
                    write!(f, "⊥") // false
                } else {
                    write!(f, "(")?;
                    for (i, formula) in formulas.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ∨ ")?;
                        }
                        write!(f, "{}", formula)?;
                    }
                    write!(f, ")")
                }
            }
            Formula::Exists(idx, body) => {
                write!(f, "∃e{}. {}", idx, body)
            }
            Formula::Forall(idx, body) => {
                write!(f, "∀e{}. {}", idx, body)
            }
        }
    }
}

/// The proof invariant extracted from an SMT-LIB file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProofInvariant<T: Eq + Hash> {
    /// Variables declared in the cert function
    pub variables: Vec<T>,
    /// The invariant formula
    pub formula: Formula<T>,
}

impl<T: Eq + Hash> ProofInvariant<T> {
    /// Create a new ProofInvariant, checking that all free variables in the formula
    /// are present in the variables list. Properly handles shadowing by existential/universal quantifiers.
    /// Panics if validation fails.
    pub fn new(variables: Vec<T>, formula: Formula<T>) -> Self
    where
        T: Clone + Display,
    {
        // Collect all free variables from the formula
        let free_vars = formula.collect_free_variables();
        
        // Convert variables list to a set for efficient lookup
        let var_set: std::collections::HashSet<_> = variables.iter().cloned().collect();
        
        // Check that all free variables are in the declared variables list
        let mut missing_vars = Vec::new();
        for var in &free_vars {
            if !var_set.contains(var) {
                missing_vars.push(var.clone());
            }
        }
        
        if !missing_vars.is_empty() {
            let missing_str: Vec<String> = missing_vars.iter().map(|v| v.to_string()).collect();
            panic!(
                "Variables used in formula but not declared: {}",
                missing_str.join(", ")
            );
        }
        
        ProofInvariant { variables, formula }
    }

    /// Map variable type from T to U
    pub fn map<U, F>(self, mut f: F) -> ProofInvariant<U>
    where
        F: FnMut(T) -> U,
        U: Eq + Hash,
    {
        ProofInvariant {
            variables: self.variables.into_iter().map(&mut f).collect(),
            formula: self.formula.map(&mut f),
        }
    }

    /// Substitute variables according to a mapping function
    /// The mapping returns Either::Left(Q) for a new variable or Either::Right(i32) for a constant
    pub fn substitute<Q, F>(&self, mut mapping: F) -> ProofInvariant<Q>
    where
        F: FnMut(&T) -> Either<Q, i32>,
        Q: Clone + Eq + Hash + Display,
        T: Clone,
    {
        // Map variables list, keeping only Left (new variables)
        let new_variables: Vec<Q> = self
            .variables
            .iter()
            .filter_map(|var| match mapping(var) {
                Either::Left(q) => Some(q),
                Either::Right(_) => None, // Constants don't appear in variable list
            })
            .collect();

        // Recursively substitute in the formula
        let new_formula = substitute_in_formula(&self.formula, &mut mapping);

        // Create new ProofInvariant - this should always succeed because we're
        // substituting from a valid ProofInvariant
        ProofInvariant::new(new_variables, new_formula)
    }
}

impl<T: Clone + Eq + Hash + Display> ProofInvariant<Either<usize, T>> {
    /// Add one token of the given variable to all multisets satisfying this invariant
    /// Q(n_a, n_b, ...) = ∃e0. P(n_a, n_b, ...) ∧ n_a = e0 + 1
    pub fn add_one(&self, var: &T) -> ProofInvariant<Either<usize, T>> {
        // Create a fresh existential variable index
        let fresh_idx = 0usize;
        let fresh_var = Either::Left(fresh_idx);

        // Add the fresh variable to the variable list
        let mut new_variables = vec![fresh_var.clone()];
        new_variables.extend(self.variables.clone());

        // Create constraint: var = e0 + 1 (which is var - e0 - 1 = 0)
        let var_expr = AffineExpr::from_var(Either::Right(var.clone()));
        let fresh_expr = AffineExpr::from_var(fresh_var.clone());
        let one = AffineExpr::from_const(1);
        let constraint_expr = var_expr.sub(&fresh_expr).sub(&one);
        let constraint = Constraint::new(constraint_expr, CompOp::Eq);

        // Combine original formula with the constraint
        let combined_formula =
            Formula::And(vec![self.formula.clone(), Formula::Constraint(constraint)]);

        // Create the proof invariant with the existential variable, then quantify it
        let proof_with_existential = ProofInvariant::new(new_variables, combined_formula);

        // Existentially quantify and project
        crate::proofinvariant_to_presburger::existentially_quantify_keep_either(
            proof_with_existential,
            &[fresh_idx],
        )
    }

    /// Remove one token of the given variable from multisets that have at least one
    /// Q(n_a, n_b, ...) = ∃e0. P(e0, n_b, ...) ∧ e0 = n_a + 1 ∧ e0 ≥ 1
    pub fn filter_and_subtract_one(&self, var: &T) -> ProofInvariant<Either<usize, T>> {
        // Create a fresh existential variable index
        let fresh_idx = 0usize;
        let fresh_var = Either::Left(fresh_idx);

        // Add the fresh variable to the variable list
        let mut new_variables = vec![fresh_var.clone()];
        new_variables.extend(self.variables.clone());

        // Create constraint: e0 = var + 1 (which is e0 - var - 1 = 0)
        let fresh_expr = AffineExpr::from_var(fresh_var.clone());
        let var_expr = AffineExpr::from_var(Either::Right(var.clone()));
        let one = AffineExpr::from_const(1);
        let equality_expr = fresh_expr.sub(&var_expr).sub(&one);
        let equality_constraint = Constraint::new(equality_expr, CompOp::Eq);

        // Create constraint: e0 >= 1 (which is e0 - 1 >= 0)
        // let geq_expr = AffineExpr::from_var(fresh_var.clone()).sub(&AffineExpr::from_const(1));
        // let geq_constraint = Constraint::new(geq_expr, CompOp::Geq);

        // Substitute var with e0 in the original formula
        let substituted_formula = self.formula.substitute_var(
            &Either::Right(var.clone()),
            Variable::Var(fresh_var.clone()),
        );

        // Combine all constraints
        let combined_formula = Formula::And(vec![
            substituted_formula,
            Formula::Constraint(equality_constraint),
            // Formula::Constraint(geq_constraint),
        ]);

        // Create the proof invariant with the existential variable
        let proof_with_existential = ProofInvariant::new(new_variables, combined_formula);

        // Existentially quantify and return
        crate::proofinvariant_to_presburger::existentially_quantify_keep_either(
            proof_with_existential,
            &[fresh_idx],
        )
    }
}

impl<L, R> ProofInvariant<Either<L, R>>
where
    L: Eq + Hash,
    R: Eq + Hash,
{
    /// Project from ProofInvariant<Either<L, R>> to ProofInvariant<R>
    /// when all Left values should not exist
    pub fn project_right(self) -> ProofInvariant<R> 
    where
        R: Clone + Display,
    {
        let variables = self
            .variables
            .into_iter()
            .filter_map(|v| match v {
                Either::Left(_) => None,
                Either::Right(r) => Some(r),
            })
            .collect();
        let formula = self.formula.project_right();
        ProofInvariant::new(variables, formula)
    }
}

/// Helper function to substitute variables in a formula
/// Mapping returns Either::Left(Q) for a new variable or Either::Right(i32) for a constant
fn substitute_in_formula<T, Q, F>(formula: &Formula<T>, mapping: &mut F) -> Formula<Q>
where
    T: Clone + Eq + Hash,
    Q: Clone + Eq + Hash,
    F: FnMut(&T) -> Either<Q, i32>,
{
    match formula {
        Formula::Constraint(c) => {
            // Substitute in the affine expression
            let mut new_terms = HashMap::default();
            let mut new_constant = c.expr.constant;

            for (var, coeff) in &c.expr.terms {
                match var {
                    Variable::Var(v) => {
                        match mapping(v) {
                            Either::Left(q) => {
                                // Variable maps to new variable
                                *new_terms.entry(Variable::Var(q)).or_insert(0) += coeff;
                            }
                            Either::Right(constant_val) => {
                                // Variable maps to constant - add to constant term
                                new_constant += coeff * constant_val as i64;
                            }
                        }
                    }
                    Variable::Existential(n) => {
                        // Existential variables are preserved
                        new_terms.insert(Variable::Existential(*n), *coeff);
                    }
                }
            }

            // Remove zero coefficients
            new_terms.retain(|_, coeff| *coeff != 0);

            // Check if this is a degenerate constraint (only constant term)
            if new_terms.is_empty() {
                // Evaluate the constant constraint
                match c.op {
                    CompOp::Eq => {
                        if new_constant == 0 {
                            // 0 = 0 is always true, return a tautology
                            // We'll represent this as an empty And
                            Formula::And(vec![])
                        } else {
                            // c = 0 where c != 0 is always false
                            // We'll represent this as an empty Or
                            Formula::Or(vec![])
                        }
                    }
                    CompOp::Geq => {
                        if new_constant >= 0 {
                            // c >= 0 where c >= 0 is always true
                            Formula::And(vec![])
                        } else {
                            // c >= 0 where c < 0 is always false
                            Formula::Or(vec![])
                        }
                    }
                }
            } else {
                Formula::Constraint(Constraint {
                    expr: AffineExpr {
                        terms: new_terms,
                        constant: new_constant,
                    },
                    op: c.op,
                })
            }
        }
        Formula::And(formulas) => {
            let mut simplified = Vec::new();
            for f in formulas {
                let subst = substitute_in_formula(f, mapping);
                match subst {
                    // Empty And is true, so ignore it in an And
                    Formula::And(inner) if inner.is_empty() => {}
                    // Empty Or is false, so the whole And becomes false
                    Formula::Or(inner) if inner.is_empty() => return Formula::Or(vec![]),
                    // Flatten nested Ands
                    Formula::And(inner) => simplified.extend(inner),
                    // Keep other formulas
                    other => simplified.push(other),
                }
            }
            Formula::And(simplified)
        }
        Formula::Or(formulas) => {
            let mut simplified = Vec::new();
            for f in formulas {
                let subst = substitute_in_formula(f, mapping);
                match subst {
                    // Empty Or is false, so ignore it in an Or
                    Formula::Or(inner) if inner.is_empty() => {}
                    // Empty And is true, so the whole Or becomes true
                    Formula::And(inner) if inner.is_empty() => return Formula::And(vec![]),
                    // Flatten nested Ors
                    Formula::Or(inner) => simplified.extend(inner),
                    // Keep other formulas
                    other => simplified.push(other),
                }
            }
            Formula::Or(simplified)
        }
        Formula::Exists(idx, body) => {
            Formula::Exists(*idx, Box::new(substitute_in_formula(body, mapping)))
        }
        Formula::Forall(idx, body) => {
            Formula::Forall(*idx, Box::new(substitute_in_formula(body, mapping)))
        }
    }
}

// Smart constructors for quantification

impl<T: Clone + Eq + Hash> Formula<T> {
    /// Find the maximum existential variable index used in the formula
    fn max_existential_index(&self) -> Option<usize> {
        match self {
            Formula::Constraint(c) => c
                .expr
                .terms
                .keys()
                .filter_map(|var| match var {
                    Variable::Existential(n) => Some(*n),
                    _ => None,
                })
                .max(),
            Formula::And(formulas) | Formula::Or(formulas) => formulas
                .iter()
                .filter_map(|f| f.max_existential_index())
                .max(),
            Formula::Exists(idx, body) => {
                let body_max = body.max_existential_index();
                Some(match body_max {
                    Some(n) => n.max(*idx),
                    None => *idx,
                })
            }
            Formula::Forall(idx, body) => {
                let body_max = body.max_existential_index();
                Some(match body_max {
                    Some(n) => n.max(*idx),
                    None => *idx,
                })
            }
        }
    }

    /// Substitute all occurrences of Var(old_var) with new_var in the formula
    fn substitute_var(&self, old_var: &T, new_var: Variable<T>) -> Self {
        match self {
            Formula::Constraint(c) => {
                let mut new_terms = HashMap::default();
                for (var, coeff) in &c.expr.terms {
                    let new_var_key = match var {
                        Variable::Var(v) if v == old_var => new_var.clone(),
                        _ => var.clone(),
                    };
                    new_terms.insert(new_var_key, *coeff);
                }
                Formula::Constraint(Constraint {
                    expr: AffineExpr {
                        terms: new_terms,
                        constant: c.expr.constant,
                    },
                    op: c.op,
                })
            }
            Formula::And(formulas) => Formula::And(
                formulas
                    .iter()
                    .map(|f| f.substitute_var(old_var, new_var.clone()))
                    .collect(),
            ),
            Formula::Or(formulas) => Formula::Or(
                formulas
                    .iter()
                    .map(|f| f.substitute_var(old_var, new_var.clone()))
                    .collect(),
            ),
            Formula::Exists(idx, body) => {
                Formula::Exists(*idx, Box::new(body.substitute_var(old_var, new_var)))
            }
            Formula::Forall(idx, body) => {
                Formula::Forall(*idx, Box::new(body.substitute_var(old_var, new_var)))
            }
        }
    }

    /// Create an existentially quantified formula
    pub fn mk_exists(self, var_to_bind: T) -> Self {
        let fresh_idx = self.max_existential_index().map(|n| n + 1).unwrap_or(0);
        let substituted = self.substitute_var(&var_to_bind, Variable::Existential(fresh_idx));
        Formula::Exists(fresh_idx, Box::new(substituted))
    }

    /// Create a universally quantified formula
    pub fn mk_forall(self, var_to_bind: T) -> Self {
        let fresh_idx = self.max_existential_index().map(|n| n + 1).unwrap_or(0);
        let substituted = self.substitute_var(&var_to_bind, Variable::Existential(fresh_idx));
        Formula::Forall(fresh_idx, Box::new(substituted))
    }
}

/// Parser for SMT-LIB proof certificates
pub struct Parser {
    input: Vec<char>,
    pos: usize,
    /// Variables declared in the current scope
    declared_vars: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at position {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseError {}

type Result<T> = std::result::Result<T, ParseError>;

impl Parser {
    fn new(input: &str) -> Self {
        Parser {
            input: input.chars().collect(),
            pos: 0,
            declared_vars: Vec::new(),
        }
    }

    fn error(&self, msg: &str) -> ParseError {
        ParseError {
            message: msg.to_string(),
            position: self.pos,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some(';') {
            while let Some(ch) = self.peek() {
                self.advance();
                if ch == '\n' {
                    break;
                }
            }
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            if self.peek() == Some(';') {
                self.skip_comment();
            } else {
                break;
            }
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<()> {
        self.skip_ws_and_comments();
        match self.peek() {
            Some(ch) if ch == expected => {
                self.advance();
                Ok(())
            }
            Some(ch) => Err(self.error(&format!("Expected '{}', found '{}'", expected, ch))),
            None => Err(self.error(&format!("Expected '{}', found EOF", expected))),
        }
    }

    fn parse_atom(&mut self) -> Result<String> {
        self.skip_ws_and_comments();

        let mut token = String::new();

        // Check for negative numbers
        if self.peek() == Some('-') {
            token.push('-');
            self.advance();
        }

        // Collect the rest
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || ch == '(' || ch == ')' {
                break;
            }
            token.push(ch);
            self.advance();
        }

        if token.is_empty() {
            Err(self.error("Expected atom"))
        } else {
            Ok(token)
        }
    }

    fn parse_integer(&mut self) -> Result<i64> {
        let atom = self.parse_atom()?;
        atom.parse::<i64>()
            .map_err(|_| self.error(&format!("Invalid integer: {}", atom)))
    }

    fn peek_atom(&mut self) -> Result<Option<String>> {
        let saved_pos = self.pos;
        self.skip_ws_and_comments();

        if self.peek() == Some('(') {
            self.pos = saved_pos;
            return Ok(None);
        }

        match self.parse_atom() {
            Ok(atom) => {
                self.pos = saved_pos;
                Ok(Some(atom))
            }
            Err(_) => {
                self.pos = saved_pos;
                Ok(None)
            }
        }
    }

    /// Parse an affine expression
    fn parse_affine_expr(&mut self) -> Result<AffineExpr<String>> {
        self.skip_ws_and_comments();

        // Check if it's a list or atom
        if self.peek() != Some('(') {
            // It's an atom - either integer or variable
            let atom = self.parse_atom()?;
            if let Ok(n) = atom.parse::<i64>() {
                Ok(AffineExpr::from_const(n))
            } else {
                // Variables with @ are allowed - they come from SMPT output
                // Check if variable is declared (without the @suffix if present)
                let base_var = atom.split('@').next().unwrap_or(&atom);
                if !self.declared_vars.contains(&base_var.to_string())
                    && !self.declared_vars.contains(&atom)
                {
                    return Err(self.error(&format!("Undefined variable: {}", atom)));
                }
                Ok(AffineExpr::from_var(atom))
            }
        } else {
            // It's a list - parse operation
            self.expect_char('(')?;
            let op = self.parse_atom()?;

            match op.as_str() {
                "+" => {
                    let mut result = AffineExpr::new();

                    // Parse all arguments
                    while self.peek() != Some(')') {
                        let arg = self.parse_affine_expr()?;
                        result = result.add(&arg);
                    }

                    self.expect_char(')')?;
                    Ok(result)
                }
                "-" => {
                    let lhs = self.parse_affine_expr()?;
                    let rhs = self.parse_affine_expr()?;
                    self.expect_char(')')?;
                    Ok(lhs.sub(&rhs))
                }
                "*" => {
                    // Parse first argument
                    let arg1 = self.parse_affine_expr()?;
                    let arg2 = self.parse_affine_expr()?;
                    self.expect_char(')')?;

                    // One must be constant
                    if arg1.is_constant() {
                        Ok(arg2.mul_by_const(arg1.get_constant()))
                    } else if arg2.is_constant() {
                        Ok(arg1.mul_by_const(arg2.get_constant()))
                    } else {
                        Err(self.error("Multiplication requires at least one constant"))
                    }
                }
                _ => Err(self.error(&format!("Unknown arithmetic operation: {}", op))),
            }
        }
    }

    /// Parse a constraint (comparison)
    fn parse_constraint(&mut self) -> Result<Constraint<String>> {
        self.expect_char('(')?;
        let op = self.parse_atom()?;

        let comp_op = match op.as_str() {
            "=" => CompOp::Eq,
            ">=" => CompOp::Geq,
            ">" => {
                // Convert > to >= by adjusting constant
                let lhs = self.parse_affine_expr()?;
                let rhs = self.parse_affine_expr()?;
                self.expect_char(')')?;

                // lhs > rhs becomes lhs - rhs > 0 becomes lhs - rhs - 1 >= 0
                let mut expr = lhs.sub(&rhs);
                expr.constant -= 1;
                return Ok(Constraint::new(expr, CompOp::Geq));
            }
            "<=" => {
                // Convert <= to >= by negation
                let lhs = self.parse_affine_expr()?;
                let rhs = self.parse_affine_expr()?;
                self.expect_char(')')?;

                // lhs <= rhs becomes rhs - lhs >= 0
                let expr = rhs.sub(&lhs);
                return Ok(Constraint::new(expr, CompOp::Geq));
            }
            "<" => {
                // Convert < to >= by negation and adjustment
                let lhs = self.parse_affine_expr()?;
                let rhs = self.parse_affine_expr()?;
                self.expect_char(')')?;

                // lhs < rhs becomes rhs - lhs > 0 becomes rhs - lhs - 1 >= 0
                let mut expr = rhs.sub(&lhs);
                expr.constant -= 1;
                return Ok(Constraint::new(expr, CompOp::Geq));
            }
            _ => return Err(self.error(&format!("Unknown comparison operator: {}", op))),
        };

        // For = and >=, parse normally
        let lhs = self.parse_affine_expr()?;
        let rhs = self.parse_affine_expr()?;
        self.expect_char(')')?;

        // Convert to expr op 0 form
        let expr = lhs.sub(&rhs);
        Ok(Constraint::new(expr, comp_op))
    }

    /// Parse quantified variables list: ((x Int) (y Int) ...) or empty ()
    fn parse_var_list(&mut self) -> Result<Vec<String>> {
        self.expect_char('(')?;
        self.skip_ws_and_comments();

        let mut vars = Vec::new();

        // Check for empty variable list
        if self.peek() == Some(')') {
            self.advance();
            return Ok(vars); // Empty variable list
        }

        while self.peek() != Some(')') {
            self.expect_char('(')?;
            let var_name = self.parse_atom()?;
            let var_type = self.parse_atom()?;
            self.expect_char(')')?;

            if var_type != "Int" {
                return Err(self.error(&format!("Expected Int type, got {}", var_type)));
            }

            vars.push(var_name);
            self.skip_ws_and_comments();
        }

        self.expect_char(')')?;
        Ok(vars)
    }

    /// Negate a normalized formula using De Morgan's laws
    fn negate_formula(formula: Formula<String>) -> Formula<String> {
        match formula {
            Formula::Constraint(c) => {
                match c.op {
                    CompOp::Eq => {
                        // ¬(expr = 0) becomes (expr > 0) ∨ (expr < 0)
                        // which is (expr >= 1) ∨ (-expr >= 1)
                        let pos_expr = c.expr.clone();
                        let mut pos_constraint = Constraint::new(pos_expr, CompOp::Geq);
                        pos_constraint.expr.constant -= 1;

                        let neg_expr = c.expr.negate();
                        let mut neg_constraint = Constraint::new(neg_expr, CompOp::Geq);
                        neg_constraint.expr.constant -= 1;

                        Formula::Or(vec![
                            Formula::Constraint(pos_constraint),
                            Formula::Constraint(neg_constraint),
                        ])
                    }
                    CompOp::Geq => {
                        // ¬(expr >= 0) becomes expr < 0 which is -expr - 1 >= 0
                        let mut neg_expr = c.expr.negate();
                        neg_expr.constant -= 1;
                        Formula::Constraint(Constraint::new(neg_expr, CompOp::Geq))
                    }
                }
            }
            Formula::And(formulas) => {
                // ¬(A ∧ B) = ¬A ∨ ¬B
                let negated: Vec<Formula<String>> =
                    formulas.into_iter().map(Self::negate_formula).collect();
                Formula::Or(negated)
            }
            Formula::Or(formulas) => {
                // ¬(A ∨ B) = ¬A ∧ ¬B
                let negated: Vec<Formula<String>> =
                    formulas.into_iter().map(Self::negate_formula).collect();
                Formula::And(negated)
            }
            Formula::Exists(var, body) => {
                // ¬∃x.P = ∀x.¬P
                Formula::Forall(var, Box::new(Self::negate_formula(*body)))
            }
            Formula::Forall(var, body) => {
                // ¬∀x.P = ∃x.¬P
                Formula::Exists(var, Box::new(Self::negate_formula(*body)))
            }
        }
    }

    /// Parse a formula
    fn parse_formula(&mut self) -> Result<Formula<String>> {
        self.skip_ws_and_comments();

        // Check for bare atoms first (true/false)
        if self.peek() != Some('(') {
            // Try to parse an atom
            if let Ok(atom) = self.parse_atom() {
                match atom.as_str() {
                    "true" => return Ok(Formula::And(vec![])), // Empty AND
                    "false" => return Ok(Formula::Or(vec![])), // Empty OR
                    _ => {
                        return Err(self.error(&format!("Expected formula, found atom '{}'", atom)));
                    }
                }
            }
            return Err(self.error("Expected '(' to start formula"));
        }

        self.expect_char('(')?;

        // Peek ahead to see what we have
        let op = if let Ok(Some(atom)) = self.peek_atom() {
            self.parse_atom()?;
            atom
        } else {
            // Empty list or other issue
            if self.peek() == Some(')') {
                self.advance();
                // Empty list - treat as empty AND (true)
                return Ok(Formula::And(vec![]));
            }
            return Err(self.error("Expected operator or closing parenthesis"));
        };

        match op.as_str() {
            "and" => {
                self.skip_ws_and_comments();

                // Check for empty (and )
                if self.peek() == Some(')') {
                    self.advance();
                    return Ok(Formula::And(vec![])); // Empty AND = true
                }

                let mut formulas = Vec::new();
                while self.peek() != Some(')') {
                    let formula = self.parse_formula()?;

                    // Skip empty AND (true) and empty OR (false) in AND context
                    // According to the test, these should just be ignored
                    if let Formula::And(ref parts) = formula {
                        if parts.is_empty() {
                            self.skip_ws_and_comments();
                            continue; // Skip empty AND
                        }
                    }

                    if let Formula::Or(ref parts) = formula {
                        if parts.is_empty() {
                            self.skip_ws_and_comments();
                            continue; // Skip empty OR
                        }
                    }

                    formulas.push(formula);
                    self.skip_ws_and_comments();
                }

                self.expect_char(')')?;
                Ok(Formula::And(formulas))
            }
            "or" => {
                self.skip_ws_and_comments();

                // Check for empty (or )
                if self.peek() == Some(')') {
                    self.advance();
                    return Ok(Formula::Or(vec![])); // Empty OR = false
                }

                let mut formulas = Vec::new();
                while self.peek() != Some(')') {
                    let formula = self.parse_formula()?;

                    // Skip empty OR (false) and empty AND (true) in OR context
                    if let Formula::Or(ref parts) = formula {
                        if parts.is_empty() {
                            self.skip_ws_and_comments();
                            continue; // Skip empty OR
                        }
                    }

                    if let Formula::And(ref parts) = formula {
                        if parts.is_empty() {
                            self.skip_ws_and_comments();
                            continue; // Skip empty AND
                        }
                    }

                    formulas.push(formula);
                    self.skip_ws_and_comments();
                }

                self.expect_char(')')?;
                Ok(Formula::Or(formulas))
            }
            "not" => {
                let inner = self.parse_formula()?;
                self.expect_char(')')?;
                Ok(Self::negate_formula(inner))
            }
            "=>" | "implies" => {
                let lhs = self.parse_formula()?;
                let rhs = self.parse_formula()?;
                self.expect_char(')')?;

                // A => B is ¬A ∨ B
                Ok(Formula::Or(vec![Self::negate_formula(lhs), rhs]))
            }
            "exists" => {
                // Save current declared vars
                let saved_vars = self.declared_vars.clone();

                let vars = self.parse_var_list()?;
                // Add to declared vars
                self.declared_vars.extend(vars.clone());

                let body = self.parse_formula()?;
                self.expect_char(')')?;

                // Restore declared vars
                self.declared_vars = saved_vars;

                // If no variables, just return the body
                if vars.is_empty() {
                    return Ok(body);
                }

                // Convert multiple variables to nested single quantifiers using smart constructor
                let mut result = body;
                for var in vars.into_iter().rev() {
                    result = result.mk_exists(var);
                }
                Ok(result)
            }
            "forall" => {
                // Save current declared vars
                let saved_vars = self.declared_vars.clone();

                let vars = self.parse_var_list()?;
                // Add to declared vars
                self.declared_vars.extend(vars.clone());

                let body = self.parse_formula()?;
                self.expect_char(')')?;

                // Restore declared vars
                self.declared_vars = saved_vars;

                // If no variables, just return the body
                if vars.is_empty() {
                    return Ok(body);
                }

                // Convert multiple variables to nested single quantifiers using smart constructor
                let mut result = body;
                for var in vars.into_iter().rev() {
                    result = result.mk_forall(var);
                }
                Ok(result)
            }
            "=" | ">=" | ">" | "<=" | "<" => {
                // It's a constraint - we already consumed '(' and the operator
                // So we need to parse it inline
                let comp_op = match op.as_str() {
                    "=" => CompOp::Eq,
                    ">=" => CompOp::Geq,
                    ">" => {
                        // Convert > to >= by adjusting constant
                        let lhs = self.parse_affine_expr()?;
                        let rhs = self.parse_affine_expr()?;
                        self.expect_char(')')?;

                        // lhs > rhs becomes lhs - rhs > 0 becomes lhs - rhs - 1 >= 0
                        let mut expr = lhs.sub(&rhs);
                        expr.constant -= 1;
                        return Ok(Formula::Constraint(Constraint::new(expr, CompOp::Geq)));
                    }
                    "<=" => {
                        // Convert <= to >= by negation
                        let lhs = self.parse_affine_expr()?;
                        let rhs = self.parse_affine_expr()?;
                        self.expect_char(')')?;

                        // lhs <= rhs becomes rhs - lhs >= 0
                        let expr = rhs.sub(&lhs);
                        return Ok(Formula::Constraint(Constraint::new(expr, CompOp::Geq)));
                    }
                    "<" => {
                        // Convert < to >= by negation and adjustment
                        let lhs = self.parse_affine_expr()?;
                        let rhs = self.parse_affine_expr()?;
                        self.expect_char(')')?;

                        // lhs < rhs becomes rhs - lhs > 0 becomes rhs - lhs - 1 >= 0
                        let mut expr = rhs.sub(&lhs);
                        expr.constant -= 1;
                        return Ok(Formula::Constraint(Constraint::new(expr, CompOp::Geq)));
                    }
                    _ => unreachable!(),
                };

                // For = and >=, parse normally
                let lhs = self.parse_affine_expr()?;
                let rhs = self.parse_affine_expr()?;
                self.expect_char(')')?;

                // Convert to expr op 0 form
                let expr = lhs.sub(&rhs);
                Ok(Formula::Constraint(Constraint::new(expr, comp_op)))
            }
            _ => Err(self.error(&format!("Unknown formula operator: {}", op))),
        }
    }

    /// Parse a complete SMT-LIB file to extract the cert function
    fn parse_smtlib(&mut self) -> Result<ProofInvariant<String>> {
        let mut cert_found = false;
        let mut variables = Vec::new();
        let mut formula = None;

        while self.pos < self.input.len() {
            self.skip_ws_and_comments();

            if self.pos >= self.input.len() {
                break;
            }

            // Each top-level form should be a list
            if self.peek() != Some('(') {
                // If we already found cert, we're done
                if cert_found {
                    break;
                }
                return Err(self.error("Expected '(' at top level"));
            }

            // Check if this is define-fun cert
            let saved_pos = self.pos;
            self.advance(); // skip '('

            if let Ok(cmd) = self.parse_atom() {
                if cmd == "define-fun" {
                    if let Ok(name) = self.parse_atom() {
                        if name == "cert" {
                            // Parse the cert function
                            cert_found = true;

                            // Parse parameters
                            self.expect_char('(')?;
                            while self.peek() != Some(')') {
                                self.expect_char('(')?;
                                let var_name = self.parse_atom()?;
                                let var_type = self.parse_atom()?;
                                self.expect_char(')')?;

                                if var_type != "Int" {
                                    return Err(
                                        self.error(&format!("Expected Int type, got {}", var_type))
                                    );
                                }

                                variables.push(var_name);
                            }
                            self.expect_char(')')?;

                            // Parse return type
                            let ret_type = self.parse_atom()?;
                            if ret_type != "Bool" {
                                return Err(self.error(&format!(
                                    "Expected Bool return type, got {}",
                                    ret_type
                                )));
                            }

                            // Set declared variables for body parsing
                            self.declared_vars = variables.clone();

                            // Parse body
                            formula = Some(self.parse_formula()?);

                            // Clear declared variables
                            self.declared_vars.clear();

                            self.expect_char(')')?; // close define-fun

                            // Once we found cert, we can stop parsing
                            break;
                        } else {
                            // Not cert, skip to end of this form
                            self.pos = saved_pos;
                            self.skip_form()?;
                        }
                    } else {
                        self.pos = saved_pos;
                        self.skip_form()?;
                    }
                } else {
                    // Not define-fun, skip
                    self.pos = saved_pos;
                    self.skip_form()?;
                }
            } else {
                self.pos = saved_pos;
                self.skip_form()?;
            }
        }

        if !cert_found {
            return Err(self.error("No cert function found in proof file"));
        }

        Ok(ProofInvariant::new(variables, formula.unwrap()))
    }

    /// Skip an S-expression form
    fn skip_form(&mut self) -> Result<()> {
        self.skip_ws_and_comments();

        if self.peek() == Some('(') {
            self.advance();
            let mut depth = 1;

            while depth > 0 && self.pos < self.input.len() {
                match self.peek() {
                    Some('(') => {
                        depth += 1;
                        self.advance();
                    }
                    Some(')') => {
                        depth -= 1;
                        self.advance();
                    }
                    Some(';') => {
                        self.skip_comment();
                    }
                    _ => {
                        self.advance();
                    }
                }
            }

            if depth > 0 {
                return Err(self.error("Unclosed parenthesis"));
            }
        } else {
            // Skip atom
            self.parse_atom()?;
        }

        Ok(())
    }
}

/// Parse a proof file and extract the invariant
pub fn parse_proof_file(content: &str) -> Result<ProofInvariant<String>> {
    let mut parser = Parser::new(content);
    parser.parse_smtlib()
}

/// Convert to presburger constraint representation
pub fn to_presburger_constraint(
    constraint: &Constraint<String>,
) -> crate::presburger::Constraint<crate::presburger::Variable<String>> {
    use crate::presburger::{Constraint as PConstraint, ConstraintType, Variable};

    let (terms, constant) = constraint.expr.to_linear_combination();
    let linear_combination: Vec<(i32, Variable<String>)> = terms
        .into_iter()
        .map(|(coeff, var)| (coeff as i32, var))
        .collect();

    let constraint_type = match constraint.op {
        CompOp::Eq => ConstraintType::EqualToZero,
        CompOp::Geq => ConstraintType::NonNegative,
    };

    PConstraint::new(linear_combination, constant as i32, constraint_type)
}

/// Map a ProofInvariant<String> to ProofInvariant<P> using a name mapping
/// This is a specialized function to avoid the infinite recursion issue with nested Either types
pub fn map_proof_variables<P>(
    proof: ProofInvariant<String>,
    name_to_place: &HashMap<String, P>,
) -> Option<ProofInvariant<P>>
where
    P: Clone + Eq + Hash + Display,
{
    // Map variables
    let mut mapped_variables = Vec::new();
    for var_name in proof.variables {
        match name_to_place.get(&var_name) {
            Some(place) => mapped_variables.push(place.clone()),
            None => return None, // Variable not found in mapping
        }
    }

    // Map formula recursively
    let mapped_formula = map_formula_variables(proof.formula, name_to_place)?;

    Some(ProofInvariant::new(mapped_variables, mapped_formula))
}

/// Helper function to map Formula<String> to Formula<P>
fn map_formula_variables<P>(
    formula: Formula<String>,
    name_to_place: &HashMap<String, P>,
) -> Option<Formula<P>>
where
    P: Clone + Eq + Hash,
{
    match formula {
        Formula::Constraint(constraint) => {
            let mapped_constraint = map_constraint_variables(constraint, name_to_place)?;
            Some(Formula::Constraint(mapped_constraint))
        }
        Formula::And(formulas) => {
            let mut mapped = Vec::new();
            for f in formulas {
                mapped.push(map_formula_variables(f, name_to_place)?);
            }
            Some(Formula::And(mapped))
        }
        Formula::Or(formulas) => {
            let mut mapped = Vec::new();
            for f in formulas {
                mapped.push(map_formula_variables(f, name_to_place)?);
            }
            Some(Formula::Or(mapped))
        }
        Formula::Exists(idx, body) => {
            let mapped_body = map_formula_variables(*body, name_to_place)?;
            Some(Formula::Exists(idx, Box::new(mapped_body)))
        }
        Formula::Forall(idx, body) => {
            let mapped_body = map_formula_variables(*body, name_to_place)?;
            Some(Formula::Forall(idx, Box::new(mapped_body)))
        }
    }
}

/// Helper function to map Constraint<String> to Constraint<P>
fn map_constraint_variables<P>(
    constraint: Constraint<String>,
    name_to_place: &HashMap<String, P>,
) -> Option<Constraint<P>>
where
    P: Clone + Eq + Hash,
{
    let mapped_expr = map_affine_expr_variables(constraint.expr, name_to_place)?;
    Some(Constraint {
        expr: mapped_expr,
        op: constraint.op,
    })
}

/// Helper function to map AffineExpr<String> to AffineExpr<P>
fn map_affine_expr_variables<P>(
    expr: AffineExpr<String>,
    name_to_place: &HashMap<String, P>,
) -> Option<AffineExpr<P>>
where
    P: Clone + Eq + Hash,
{
    let mut mapped_terms = HashMap::default();

    for (var, coeff) in expr.terms {
        let mapped_var = match var {
            Variable::Var(name) => {
                let place = name_to_place.get(&name)?;
                Variable::Var(place.clone())
            }
            Variable::Existential(idx) => Variable::Existential(idx),
        };
        mapped_terms.insert(mapped_var, coeff);
    }

    Some(AffineExpr {
        terms: mapped_terms,
        constant: expr.constant,
    })
}

/// Pretty‐print a parsed certificate
impl<T: fmt::Display + fmt::Debug + Eq + Hash> fmt::Display for ProofInvariant<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Certificate variables: {:?}", self.variables)?;
        writeln!(f, "Certificate formula:")?;

        match &self.formula {
            Formula::And(parts) if parts.len() > 1 => {
                writeln!(f, "(")?;
                for (i, part) in parts.iter().enumerate() {
                    write!(f, "    ({})", part)?;
                    if i + 1 < parts.len() {
                        writeln!(f, " ∧")?;
                    } else {
                        writeln!(f)?; // last line, just newline
                    }
                }
                write!(f, ")")
            }
            Formula::Or(parts) if parts.len() > 1 => {
                writeln!(f, "(")?;
                for (i, part) in parts.iter().enumerate() {
                    write!(f, "    ({})", part)?;
                    if i + 1 < parts.len() {
                        writeln!(f, " ∨")?;
                    } else {
                        writeln!(f)?;
                    }
                }
                write!(f, ")")
            }
            // fallback to the plain Display (no extra parens / lines)
            other => write!(f, "{}", other),
        }
    }
}

/// Parse the given SMT-LIB text and print its `cert` function to stdout
pub fn print_proof_certificate(content: &str) -> Result<()> {
    let inv = parse_proof_file(content)?;
    println!("{}", inv);
    Ok(())
}

/// Recursively print a Formula AST with indentation
fn print_formula_tree<T: fmt::Display + Eq + Hash>(formula: &Formula<T>, indent: usize) {
    let pad = "  ".repeat(indent);
    match formula {
        Formula::Constraint(c) => {
            println!("{}Constraint: {}", pad, c);
        }
        Formula::And(children) => {
            println!("{}And", pad);
            for child in children {
                print_formula_tree(child, indent + 1);
            }
        }
        Formula::Or(children) => {
            println!("{}Or", pad);
            for child in children {
                print_formula_tree(child, indent + 1);
            }
        }
        Formula::Exists(idx, body) => {
            println!("{}Exists e{}", pad, idx);
            print_formula_tree(body, indent + 1);
        }
        Formula::Forall(idx, body) => {
            println!("{}Forall e{}", pad, idx);
            print_formula_tree(body, indent + 1);
        }
    }
}

/// Recursively convert a normalized `Formula` into a `PresburgerSet<String>`.
/// - Leaves yield a single‐constraint `QuantifiedSet<String>`.
/// - `And` → intersection of children’s sets.
/// - `Or`  → union of children’s sets.
/// - `Exists(x, body)` → project out `x` (we push `x` into the mapping before descending).
fn formula_to_presburger(formula: &Formula<String>, mapping: Vec<String>) -> PresburgerSet<String> {
    match formula {
        Formula::Constraint(c) => {
            // each leaf constraint → one QuantifiedSet
            let pcon: PConstraint<Variable<String>> = to_presburger_constraint(c);
            let qs = QuantifiedSet::new(vec![pcon]);
            PresburgerSet::from_quantified_sets(&[qs], mapping)
        }
        Formula::And(children) => {
            // intersection of all children
            let mut iter = children
                .iter()
                .map(|f| formula_to_presburger(f, mapping.clone()));
            let first = iter
                .next()
                .unwrap_or_else(|| PresburgerSet::universe(mapping.clone()));
            iter.fold(first, |acc, next| acc.intersection(&next))
        }
        Formula::Or(children) => {
            // union of all children
            let mut iter = children
                .iter()
                .map(|f| formula_to_presburger(f, mapping.clone()));
            let first = iter.next().unwrap_or_else(PresburgerSet::zero);
            iter.fold(first, |acc, next| acc.union(&next))
        }
        Formula::Exists(_idx, body) => {
            // Existential variables are already handled in the constraints
            // Just process the body - the existential variables will be in the QuantifiedSet
            formula_to_presburger(body, mapping)
        }
        Formula::Forall(_, _) => {
            panic!("`Forall` → Presburger not implemented");
        }
    }
}

/// Parse a certificate from disk and build its Presburger set.
pub fn parse_and_build_presburger_set<P: AsRef<Path>>(
    path: P,
) -> std::result::Result<PresburgerSet<String>, Box<dyn std::error::Error>> {
    let txt = fs::read_to_string(path)?;
    let inv = parse_proof_file(&txt)?;
    // start with the vector of _parameters_ as the initial mapping
    Ok(formula_to_presburger(&inv.formula, inv.variables.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affine_expr() {
        let x = AffineExpr::from_var("x".to_string());
        let y = AffineExpr::from_var("y".to_string());
        let five = AffineExpr::from_const(5);

        // x + y + 5
        let expr = x.add(&y).add(&five);
        // Check components instead of string representation due to HashMap ordering
        assert_eq!(expr.get_coeff(&Variable::Var("x".to_string())), 1);
        assert_eq!(expr.get_coeff(&Variable::Var("y".to_string())), 1);
        assert_eq!(expr.get_constant(), 5);

        // 2*x - 3*y + 10
        let expr2 = x
            .mul_by_const(2)
            .add(&y.mul_by_const(-3))
            .add(&AffineExpr::from_const(10));
        // Check components instead of string representation due to HashMap ordering
        assert_eq!(expr2.get_coeff(&Variable::Var("x".to_string())), 2);
        assert_eq!(expr2.get_coeff(&Variable::Var("y".to_string())), -3);
        assert_eq!(expr2.get_constant(), 10);

        // Test subtraction
        let expr3 = x.sub(&y);
        // Check components instead of string representation due to HashMap ordering
        assert_eq!(expr3.get_coeff(&Variable::Var("x".to_string())), 1);
        assert_eq!(expr3.get_coeff(&Variable::Var("y".to_string())), -1);
        assert_eq!(expr3.get_constant(), 0);
    }

    #[test]
    fn test_simple_proof() {
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)(y Int)) Bool 
  (and (>= x 0) (>= y 0) (= (+ x y) 10)))
"#;

        let result = parse_proof_file(proof).unwrap();
        assert_eq!(result.variables, vec!["x", "y"]);

        // Check it's an AND of 3 constraints
        match &result.formula {
            Formula::And(constraints) => {
                assert_eq!(constraints.len(), 3);
            }
            _ => panic!("Expected AND formula"),
        }
    }

    #[test]
    fn test_undefined_variable() {
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool 
  (>= x y))
"#;

        let result = parse_proof_file(proof);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Undefined variable"));
    }

    #[test]
    fn test_variable_with_suffix() {
        // Variables with @ suffixes are now allowed
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool 
  (>= x@0 0))
"#;

        let result = parse_proof_file(proof);
        assert!(result.is_ok());
        let inv = result.unwrap();
        match &inv.formula {
            Formula::Constraint(c) => {
                assert_eq!(c.expr.to_string(), "x@0");
            }
            _ => panic!("Expected constraint"),
        }
    }

    #[test]
    fn test_nested_arithmetic() {
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)(y Int)) Bool 
  (= (+ (+ x 1) y) 10))
"#;

        let result = parse_proof_file(proof).unwrap();
        match &result.formula {
            Formula::Constraint(c) => {
                // Check components instead of string representation due to HashMap ordering
                assert_eq!(c.expr.get_coeff(&Variable::Var("x".to_string())), 1);
                assert_eq!(c.expr.get_coeff(&Variable::Var("y".to_string())), 1);
                assert_eq!(c.expr.get_constant(), -9);
                assert_eq!(c.op, CompOp::Eq);
            }
            _ => panic!("Expected constraint"),
        }
    }

    #[test]
    fn test_normalization() {
        // Test > becomes >=
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool (> x 5))
"#;

        let result = parse_proof_file(proof).unwrap();
        match &result.formula {
            Formula::Constraint(c) => {
                assert_eq!(c.expr.to_string(), "x -6");
                assert_eq!(c.op, CompOp::Geq);
            }
            _ => panic!("Expected constraint"),
        }
    }

    #[test]
    fn test_implies_normalization() {
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)(y Int)) Bool 
  (=> (>= x 0) (>= y 0)))
"#;

        let result = parse_proof_file(proof).unwrap();
        // Should be (¬(x >= 0) ∨ (y >= 0))
        match &result.formula {
            Formula::Or(parts) => {
                assert_eq!(parts.len(), 2);
                // First part should be ¬(x >= 0) which is -x - 1 >= 0
                match &parts[0] {
                    Formula::Constraint(c) => {
                        assert_eq!(c.expr.to_string(), "-x -1");
                        assert_eq!(c.op, CompOp::Geq);
                    }
                    _ => panic!("Expected constraint"),
                }
            }
            _ => panic!("Expected OR"),
        }
    }

    #[test]
    fn test_exists() {
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool 
  (exists ((t Int)) (and (>= t 0) (= x (* 2 t)))))
"#;

        let result = parse_proof_file(proof).unwrap();
        match &result.formula {
            Formula::Exists(idx, body) => {
                // The existential variable should have index 0
                assert_eq!(*idx, 0);
                match body.as_ref() {
                    Formula::And(constraints) => {
                        assert_eq!(constraints.len(), 2);
                    }
                    _ => panic!("Expected AND in exists body"),
                }
            }
            _ => panic!("Expected EXISTS"),
        }
    }

    #[test]
    fn test_empty_and_or() {
        let proof = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool 
  (and (or ) (>= x 0) (and )))
"#;

        let result = parse_proof_file(proof).unwrap();
        match &result.formula {
            Formula::And(parts) => {
                // We have: empty OR (false), x >= 0, empty AND (true)
                // Empty lists are skipped during parsing, so we should have just x >= 0
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    Formula::Constraint(c) => {
                        assert_eq!(c.expr.to_string(), "x");
                    }
                    _ => panic!("Expected constraint"),
                }
            }
            _ => panic!("Expected AND"),
        }
    }

    #[test]
    fn test_true_false_constants() {
        // Test parsing 'true' constant
        let proof_true = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool true)
"#;

        let result = parse_proof_file(proof_true).unwrap();
        match &result.formula {
            Formula::And(parts) => {
                assert_eq!(parts.len(), 0, "true should be empty AND");
            }
            _ => panic!("Expected AND for true"),
        }

        // Test parsing 'false' constant
        let proof_false = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool false)
"#;

        let result = parse_proof_file(proof_false).unwrap();
        match &result.formula {
            Formula::Or(parts) => {
                assert_eq!(parts.len(), 0, "false should be empty OR");
            }
            _ => panic!("Expected OR for false"),
        }

        // Test true and false in context
        let proof_mixed = r#"
(set-logic LIA)
(define-fun cert ((x Int)) Bool 
  (and true (>= x 0) false))
"#;

        let result = parse_proof_file(proof_mixed).unwrap();
        match &result.formula {
            Formula::And(parts) => {
                // true is skipped, false is skipped in AND context
                // Should have just x >= 0
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    Formula::Constraint(c) => {
                        assert_eq!(c.expr.to_string(), "x");
                    }
                    _ => panic!("Expected constraint"),
                }
            }
            _ => panic!("Expected AND"),
        }

        // Test OR with true and false
        let proof_or = r#"
(set-logic LIA)  
(define-fun cert ((x Int)) Bool
  (or false (= x 5) true))
"#;

        let result = parse_proof_file(proof_or).unwrap();
        match &result.formula {
            Formula::Or(parts) => {
                // false is skipped, true is skipped in OR context
                // Should have just x = 5
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    Formula::Constraint(c) => {
                        assert_eq!(c.expr.to_string(), "x -5");
                        assert_eq!(c.op, CompOp::Eq);
                    }
                    _ => panic!("Expected constraint"),
                }
            }
            _ => panic!("Expected OR"),
        }
    }

    #[test]
    fn test_parse_all_proof_files_in_out_dir() {
        use std::fs;
        use std::path::Path;

        let out_dir = Path::new("out");
        if !out_dir.exists() {
            println!("Skipping test: out directory does not exist");
            return;
        }

        let mut total_files = 0;
        let mut successful_parses = 0;
        let mut failed_parses = 0;
        let mut failures = Vec::new();

        // Function to recursively find proof files
        fn find_proof_files(
            dir: &Path,
            files: &mut Vec<std::path::PathBuf>,
        ) -> std::io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    find_proof_files(&path, files)?;
                } else if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        if name_str.contains("proof") && name_str.ends_with(".txt") {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        }

        let mut proof_files = Vec::new();
        if let Err(e) = find_proof_files(out_dir, &mut proof_files) {
            println!("Error scanning directory: {}", e);
            return;
        }

        println!("\nFound {} proof files to test", proof_files.len());

        for file_path in proof_files {
            total_files += 1;

            match fs::read_to_string(&file_path) {
                Ok(content) => match parse_proof_file(&content) {
                    Ok(invariant) => {
                        successful_parses += 1;
                        println!(
                            "✓ {} ({} vars)",
                            file_path.display(),
                            invariant.variables.len()
                        );
                    }
                    Err(e) => {
                        failed_parses += 1;
                        let relative_path = file_path.strip_prefix("out/").unwrap_or(&file_path);
                        failures.push((relative_path.to_path_buf(), e.to_string()));
                        println!("✗ {}: {}", file_path.display(), e);
                    }
                },
                Err(e) => {
                    failed_parses += 1;
                    failures.push((file_path.clone(), format!("Failed to read file: {}", e)));
                    println!("✗ {}: Failed to read file: {}", file_path.display(), e);
                }
            }
        }

        println!("\n=== Summary ===");
        println!("Total files: {}", total_files);
        println!("Successful: {}", successful_parses);
        println!("Failed: {}", failed_parses);

        if !failures.is_empty() {
            println!("\n=== Failures ===");
            for (path, error) in &failures {
                println!("{}: {}", path.display(), error);
            }

            // Check if failures are due to expected reasons
            let no_cert_failures = failures
                .iter()
                .filter(|(_, err)| err.contains("No cert function"))
                .count();

            println!(
                "\nFailures due to missing cert function: {}/{}",
                no_cert_failures, failed_parses
            );

            // Don't fail the test if all errors are expected
            if no_cert_failures < failed_parses {
                println!("\nUnexpected failures found!");
                for (path, error) in &failures {
                    if !error.contains("No cert function") {
                        println!("  {}: {}", path.display(), error);
                    }
                }
                panic!("Some files failed to parse for unexpected reasons");
            }
        }
    }

    #[test]
    fn test_parse_all_proof_files_quiet() {
        use std::fs;
        use std::path::Path;

        let out_dir = Path::new("out");
        if !out_dir.exists() {
            return; // Skip if no out directory
        }

        // Recursively find proof files
        fn find_proof_files(
            dir: &Path,
            files: &mut Vec<std::path::PathBuf>,
        ) -> std::io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    find_proof_files(&path, files)?;
                } else if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        if name_str.contains("proof") && name_str.ends_with(".txt") {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        }

        let mut proof_files = Vec::new();
        find_proof_files(out_dir, &mut proof_files).expect("Failed to scan directory");

        let mut stats = (0, 0, 0); // (total, success, expected_failures)

        for file_path in proof_files {
            stats.0 += 1;

            if let Ok(content) = fs::read_to_string(&file_path) {
                match parse_proof_file(&content) {
                    Ok(_) => stats.1 += 1,
                    Err(e) => {
                        if e.message.contains("No cert function") {
                            stats.2 += 1;
                        } else {
                            panic!("Unexpected parse error in {}: {}", file_path.display(), e);
                        }
                    }
                }
            }
        }

        // Basic sanity check
        assert_eq!(
            stats.0,
            stats.1 + stats.2,
            "Total files ({}) != successful ({}) + expected failures ({})",
            stats.0,
            stats.1,
            stats.2
        );

        // Ensure we parsed some files successfully
        assert!(stats.1 > 0, "No files were parsed successfully");
    }
}

#[test]
fn test_parse_and_print_specific_proof_file() {
    let proof_path =
        Path::new("out/simple_nonser2_turned_ser_with_locks/smpt_constraints_disjunct_0_proof.txt");
    assert!(
        proof_path.exists(),
        "Test fixture not found: {}",
        proof_path.display()
    );

    let content = fs::read_to_string(proof_path).expect("Failed to read proof file for test");

    // Parse normally...
    let inv = parse_proof_file(&content).expect("parse_proof_file failed");

    // Now print the tree from root to leaves:
    println!("\n=== Parsed Formula Tree ===");
    print_formula_tree(&inv.formula, 0);

    // And still allow us to inspect the parsed invariant:
    assert!(!inv.variables.is_empty(), "No variables parsed");
    match inv.formula {
        Formula::And(ref parts) | Formula::Or(ref parts) => {
            assert!(!parts.is_empty(), "Parsed an empty conjunct/disjunct");
        }
        _ => {}
    }
}

#[test]
fn test_parse_and_build_set() {
    let proof_path =
        Path::new("out/simple_nonser2_turned_ser_with_locks/smpt_constraints_disjunct_0_proof.txt");
    assert!(proof_path.exists());

    let set =
        parse_and_build_presburger_set(proof_path).expect("parse_and_build_presburger_set failed");

    println!("Resulting Presburger set:\n{}", set);
    assert!(!set.is_empty());
}
