use std::fmt::{self, Display, Formatter};

use indexmap::IndexMap;
use itertools::Itertools;
use rustc_hash::FxBuildHasher;

use crate::{MarkerExpression, MarkerTree};

/// Write a marker with repeated conditions factored out.
pub(super) fn write_marker(marker: MarkerTree, f: &mut Formatter<'_>) -> fmt::Result {
    let clauses = marker.to_dnf();
    if let Some(factored) = factor(&clauses) {
        return f.write_str(&factored);
    }
    write!(f, "{}", disjunction(&clauses))
}

/// Group clauses by their shared prefix, then combine prefixes with the same suffixes.
/// For example, AX + AY + BX + BY becomes (A + B)(X + Y).
fn factor(clauses: &[Vec<MarkerExpression>]) -> Option<String> {
    if clauses.len() < 2 || clauses.iter().all(|clause| clause.len() == 1) {
        return None;
    }

    let mut by_first = IndexMap::<_, Vec<_>, FxBuildHasher>::default();
    for clause in clauses {
        by_first
            .entry(clause.first()?)
            .or_default()
            .push(clause.as_slice());
    }

    let shared_prefix = by_first.len() < clauses.len();
    let mut by_suffixes = IndexMap::<_, Vec<_>, FxBuildHasher>::default();
    for group in by_first.into_values() {
        let first = group[0];
        let prefix_len = if group.len() == 1 {
            1
        } else {
            group[1..].iter().fold(first.len(), |len, clause| {
                first[..len]
                    .iter()
                    .zip(*clause)
                    .take_while(|(a, b)| a == b)
                    .count()
            })
        };
        let mut suffixes = group
            .iter()
            .map(|clause| &clause[prefix_len..])
            .collect::<Vec<_>>();
        if suffixes.iter().any(|suffix| suffix.is_empty()) {
            // The prefix alone covers every longer clause in this group.
            suffixes.clear();
        }
        let prefixes = by_suffixes.entry(suffixes).or_default();
        prefixes.push(&first[..prefix_len]);
    }
    let shared_suffix = by_suffixes
        .iter()
        .any(|(suffixes, prefixes)| !suffixes.is_empty() && prefixes.len() > 1);
    (shared_prefix || shared_suffix).then(|| {
        by_suffixes
            .into_iter()
            .format_with(" or ", |(suffixes, prefixes), f| {
                if suffixes.is_empty() {
                    return f(&disjunction(&prefixes));
                }
                f(&format_args!(
                    "{} and {}",
                    operand(&prefixes),
                    operand(&suffixes)
                ))
            })
            .to_string()
    })
}

/// Parenthesize a disjunction when it is an operand of `and`.
fn operand(clauses: &[&[MarkerExpression]]) -> impl Display {
    std::fmt::from_fn(move |f| {
        if clauses.len() == 1 {
            write!(f, "{}", disjunction(clauses))
        } else {
            write!(f, "({})", disjunction(clauses))
        }
    })
}

/// Render a disjunction of conjunctions in the same order as the extracted cover.
fn disjunction<T: AsRef<[MarkerExpression]>>(clauses: &[T]) -> impl Display {
    clauses.iter().format_with(" or ", |clause, f| {
        let terms = clause.as_ref();
        let conjunction = terms.iter().format(" and ");
        if clauses.len() > 1 && terms.len() > 1 {
            f(&format_args!("({conjunction})"))
        } else {
            f(&conjunction)
        }
    })
}
