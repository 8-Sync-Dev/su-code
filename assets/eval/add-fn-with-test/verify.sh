#!/bin/sh
# $1 = omp stdout capture (unused). The verifier OWNS the assertion so the agent
# cannot game the check by writing its own passing test.
cat >> src/lib.rs <<'EOF'

#[cfg(test)]
mod _eval_check {
    use super::*;
    #[test]
    fn slug() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("A B!c"), "a-bc");
        assert_eq!(slugify("rust_LANG 2026"), "rustlang-2026");
    }
}
EOF
cargo test 2>&1 | grep -q 'test result: ok'
