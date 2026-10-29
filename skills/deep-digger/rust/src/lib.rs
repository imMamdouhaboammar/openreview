//! Dependency-free line-oriented delta debugging core.
//!
//! This proves 1-minimality relative to a deterministic oracle, not a unique
//! or ultimate causal explanation. Unresolved results cannot prove failure.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Unresolved,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Minimization {
    pub lines: Vec<String>,
    pub tests: usize,
    pub complete: bool,
}

pub fn ddmin<F>(initial: &[String], max_tests: usize, mut oracle: F) -> Result<Minimization, String>
where
    F: FnMut(&[String]) -> Verdict,
{
    if max_tests == 0 {
        return Err("max_tests must be positive".into());
    }
    let mut tests = 0usize;
    let mut check = |lines: &[String]| -> Option<Verdict> {
        if tests == max_tests {
            return None;
        }
        tests += 1;
        Some(oracle(lines))
    };
    if check(initial) != Some(Verdict::Fail) {
        return Err("baseline must deterministically fail".into());
    }
    let mut current = initial.to_vec();
    let mut granularity = 2usize;
    while current.len() >= 2 {
        let stride = current.len().div_ceil(granularity);
        let mut reduced = false;
        for start in (0..current.len()).step_by(stride) {
            let end = (start + stride).min(current.len());
            let candidate: Vec<_> = current[..start]
                .iter()
                .chain(current[end..].iter())
                .cloned()
                .collect();
            match check(&candidate) {
                Some(Verdict::Fail) => {
                    current = candidate;
                    granularity = granularity.saturating_sub(1).max(2);
                    reduced = true;
                    break;
                }
                None => {
                    return Ok(Minimization {
                        lines: current,
                        tests,
                        complete: false,
                    });
                }
                _ => {}
            }
        }
        if reduced {
            continue;
        }
        if granularity >= current.len() {
            break;
        }
        granularity = (granularity * 2).min(current.len());
    }
    for index in 0..current.len() {
        let candidate: Vec<_> = current
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, line)| line.clone())
            .collect();
        if check(&candidate) != Some(Verdict::Pass) {
            // A mixed/incomplete oracle or budget exhaustion cannot certify 1-minimality.
            return Ok(Minimization {
                lines: current,
                tests,
                complete: false,
            });
        }
    }
    Ok(Minimization {
        lines: current,
        tests,
        complete: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<String> {
        ["a\n", "BUG\n", "noise\n", "TRIGGER\n", "z\n"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn interaction_is_one_minimal() {
        let result = ddmin(&lines(), 100, |chunks| {
            if chunks.iter().any(|x| x == "BUG\n") && chunks.iter().any(|x| x == "TRIGGER\n") {
                Verdict::Fail
            } else {
                Verdict::Pass
            }
        })
        .unwrap();
        assert!(result.complete);
        assert_eq!(result.lines, vec!["BUG\n", "TRIGGER\n"]);
    }

    #[test]
    fn budget_exhaustion_is_incomplete() {
        let result = ddmin(&lines(), 1, |_| Verdict::Fail).unwrap();
        assert!(!result.complete);
        assert_eq!(result.lines.len(), 5);
    }

    #[test]
    fn baseline_must_fail() {
        assert!(ddmin(&lines(), 10, |_| Verdict::Pass).is_err());
    }

    #[test]
    fn unresolved_cannot_certify_minimality() {
        let result = ddmin(&lines(), 100, |chunks| {
            if chunks == lines() {
                Verdict::Fail
            } else {
                Verdict::Unresolved
            }
        })
        .unwrap();
        assert!(!result.complete);
    }

    #[test]
    fn empty_failing_input_can_be_minimal() {
        let result = ddmin(&[], 10, |_| Verdict::Fail).unwrap();
        assert!(result.complete);
    }
}
