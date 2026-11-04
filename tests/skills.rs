//! The agent skills in `skills/` must describe the API that actually ships.
//! Each check here fails when code and skill drift apart, so updating one
//! forces updating the other:
//! - every spec geom and layer param is documented in the CLI and JS skills,
//!   and every geom they list exists;
//! - every CLI flag the CLI skill mentions exists, and every flag is documented;
//! - every `geom_*` builder the Rust skill mentions exists;
//! - the Rust skill's `ggplot-rs = "X.Y"` pin matches this crate's version.
//!
//! The Rust skill's code blocks are additionally compiled as doctests (see
//! `RustSkillExamples` in src/lib.rs) and the JS skill's examples run under
//! vitest (js/tests/skill.test.ts).
#![cfg(feature = "spec")]

use std::collections::BTreeSet;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The text of the `## {title}` section.
fn section<'a>(md: &'a str, title: &str) -> &'a str {
    let head = format!("\n## {title}\n");
    let a = md
        .find(&head)
        .unwrap_or_else(|| panic!("no '## {title}' section"))
        + head.len();
    let b = md[a..].find("\n## ").map_or(md.len(), |i| a + i);
    &md[a..b]
}

/// Geom names listed in a skill's `## Geoms` section: the first backtick span
/// of each bullet (bullets may wrap onto indented continuation lines).
fn listed_geoms(md: &str) -> BTreeSet<String> {
    let mut bullets: Vec<String> = Vec::new();
    for line in section(md, "Geoms").lines() {
        if let Some(rest) = line.strip_prefix("- ") {
            bullets.push(rest.to_string());
        } else if line.starts_with("  ") {
            if let Some(b) = bullets.last_mut() {
                b.push(' ');
                b.push_str(line.trim());
            }
        } else if !line.trim().is_empty() {
            bullets.push(String::new()); // a paragraph ends the bullet list
        }
    }
    bullets
        .iter()
        .filter_map(|b| b.split('`').nth(1))
        .flat_map(|span| span.split_whitespace().map(str::to_string))
        .collect()
}

fn layer_params() -> Vec<String> {
    let src = read("src/spec.rs");
    let body = &src[src.find("pub struct LayerParams {").unwrap()..];
    let body = &body[..body.find("\n}").unwrap()];
    body.lines()
        .skip(1)
        .filter_map(|l| l.trim().strip_prefix("pub "))
        .filter_map(|l| l.split(':').next())
        .map(str::to_string)
        .collect()
}

fn check_spec_skill(path: &str) {
    let md = read(path);
    let listed = listed_geoms(&md);
    let spec: BTreeSet<String> = ggplot_rs::spec::GEOMS
        .iter()
        .map(|g| g.to_string())
        .collect();
    let undocumented: Vec<_> = spec.difference(&listed).collect();
    let stale: Vec<_> = listed.difference(&spec).collect();
    assert!(
        undocumented.is_empty(),
        "{path}: geoms missing from '## Geoms': {undocumented:?}"
    );
    assert!(
        stale.is_empty(),
        "{path}: '## Geoms' lists unknown geoms: {stale:?}"
    );
    for p in layer_params() {
        assert!(
            md.contains(&format!("`{p}")),
            "{path}: layer param `{p}` undocumented"
        );
    }
}

#[test]
fn cli_skill_covers_every_geom_and_param() {
    check_spec_skill("skills/ggplot-rs-cli/SKILL.md");
}

#[test]
fn js_skill_covers_every_geom_and_param() {
    check_spec_skill("skills/ggplot-rs-js/SKILL.md");
}

/// `--flag` names the CLI's clap `Args` struct defines.
fn cli_flags() -> BTreeSet<String> {
    let src = read("src/bin/ggplot.rs");
    let body = &src[src.find("struct Args {").unwrap()..];
    let body = &body[..body.find("\n}").unwrap()];
    let mut flags = BTreeSet::from(["--help".to_string(), "--version".to_string()]);
    let mut long_override = None;
    for line in body.lines().map(str::trim) {
        if line.starts_with("#[arg(") {
            long_override = line
                .split("long = \"")
                .nth(1)
                .and_then(|r| r.split('"').next())
                .map(str::to_string);
        } else if let Some((field, _)) = line.split_once(": ") {
            if !field.starts_with("//") && !field.contains(' ') {
                let name = long_override
                    .take()
                    .unwrap_or_else(|| field.replace('_', "-"));
                flags.insert(format!("--{name}"));
            }
        }
    }
    flags
}

#[test]
fn cli_skill_flags_match_the_binary() {
    let md = read("skills/ggplot-rs-cli/SKILL.md");
    let flags = cli_flags();
    assert!(
        flags.contains("--facet-wrap") && flags.contains("--aes"),
        "{flags:?}"
    );
    let mut mentioned = BTreeSet::new();
    for (i, _) in md.match_indices("--") {
        let rest = &md[i + 2..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '-')
            .collect();
        // Table rules (`|------|`) and wildcards (`--facet-*`) aren't flags.
        let name = name.trim_end_matches('-');
        if name.is_empty() || rest[name.len()..].starts_with("-*") {
            continue;
        }
        // `cargo install --features cli` etc. are cargo's flags, not ours.
        let before = &md[..i];
        let line = &before[before.rfind('\n').map_or(0, |n| n + 1)..];
        if !line.trim_start().starts_with("cargo ") {
            mentioned.insert(format!("--{name}"));
        }
    }
    let unknown: Vec<_> = mentioned.difference(&flags).collect();
    assert!(
        unknown.is_empty(),
        "CLI skill mentions flags the binary lacks: {unknown:?}"
    );
    let undocumented: Vec<_> = flags
        .iter()
        .filter(|f| !["--help", "--version"].contains(&f.as_str()))
        .filter(|f| !mentioned.contains(*f))
        .collect();
    assert!(
        undocumented.is_empty(),
        "CLI flags missing from the skill: {undocumented:?}"
    );
}

#[test]
fn rust_skill_names_real_builders_and_current_version() {
    let md = read("skills/ggplot-rs/SKILL.md");
    let mut src = String::new();
    fn walk(dir: &std::path::Path, out: &mut String) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push_str(&std::fs::read_to_string(p).unwrap());
            }
        }
    }
    walk(&std::path::Path::new(ROOT).join("src"), &mut src);
    for (i, _) in md.match_indices("geom_") {
        let name: String = md[i..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if md[i + name.len()..].starts_with('*') {
            continue; // `geom_*` wildcard
        }
        let name = name.trim_end_matches('_');
        assert!(
            src.contains(&format!("pub fn {name}(")) || src.contains(&format!("pub fn {name}<")),
            "Rust skill mentions `{name}`, which is not a builder"
        );
    }
    let v = env!("CARGO_PKG_VERSION");
    let minor = v.rsplit_once('.').unwrap().0;
    assert!(
        md.contains(&format!("ggplot-rs = \"{minor}\"")),
        "Rust skill's Cargo.toml pin must be ggplot-rs = \"{minor}\" (crate is {v})"
    );
}
