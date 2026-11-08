//! Argument parsing from command id modifiers and argument strings (task 2.3).
//!
//! Produces a `Request` holding the target spec (if any), insertion flag, and
//! remaining positional tokens, validating spec names and insertion capabilities.

use crate::registry::{CommandSpec, INSERT, ParsedId, Positional, is_spec_name};

/// Parsed command invocation request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Request {
    pub spec: Option<String>,
    pub insert: bool,
    pub rest: Vec<String>,
    pub raw: Option<String>,
}

/// Parses the invocation request combining command declaration, parsed id, and arguments.
pub fn parse(cmd: &CommandSpec, parsed: &ParsedId, arg: Option<&str>) -> Result<Request, String> {
    let mut insert = parsed.insert;
    let raw = arg.map(str::to_owned);

    let raw_tokens: Vec<&str> = arg.unwrap_or_default().split_whitespace().collect();
    let mut filtered_tokens = Vec::new();

    for token in raw_tokens {
        if token == INSERT {
            insert = true;
        } else {
            filtered_tokens.push(token);
        }
    }

    if insert && !cmd.insert_capable {
        return Err(format!("`{}` does not support insertion", cmd.id()));
    }

    let id_spec = parsed.spec.as_deref();
    let mut arg_spec = None;
    let rest: Vec<String>;

    if cmd.positional == Positional::Spec {
        if let Some(first) = filtered_tokens.first() {
            arg_spec = Some(*first);
            rest = filtered_tokens
                .get(1..)
                .unwrap_or_default()
                .iter()
                .map(|&s| s.to_owned())
                .collect();
        } else {
            rest = Vec::new();
        }
    } else {
        rest = filtered_tokens.iter().map(|&s| s.to_owned()).collect();
    }

    let chosen_spec = match (id_spec, arg_spec) {
        (Some(a), Some(b)) => {
            if a != b {
                return Err(format!(
                    "spec named twice: `{a}` (command) and `{b}` (argument)"
                ));
            }
            Some(a)
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    if let Some(spec) = chosen_spec
        && !is_spec_name(spec)
    {
        return Err(format!(
            "invalid spec name `{spec}`: use a-z, 0-9, '.', '-', '_' without path separators"
        ));
    }

    Ok(Request {
        spec: chosen_spec.map(str::to_owned),
        insert,
        rest,
        raw,
    })
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Group;

    fn spec_cmd() -> CommandSpec {
        CommandSpec {
            group: Group::Ws,
            name: "current",
            title: "Kiro: Current Spec",
            spec_addressable: true,
            insert_capable: false,
            positional: Positional::Spec,
        }
    }

    fn list_cmd() -> CommandSpec {
        CommandSpec {
            group: Group::Ws,
            name: "specs",
            title: "Kiro: List Specs",
            spec_addressable: false,
            insert_capable: true,
            positional: Positional::None,
        }
    }

    fn pass_through_cmd() -> CommandSpec {
        CommandSpec {
            group: Group::Vocab,
            name: "term",
            title: "Kiro: Lookup Term",
            spec_addressable: true,
            insert_capable: false,
            positional: Positional::PassThrough,
        }
    }

    #[test]
    fn parse_no_argument() {
        let cmd = list_cmd();
        let parsed = ParsedId {
            base: "ws.specs".into(),
            insert: false,
            spec: None,
        };

        let req = parse(&cmd, &parsed, None).unwrap();
        assert_eq!(req.spec, None);
        assert!(!req.insert);
        assert!(req.rest.is_empty());
        assert_eq!(req.raw, None);
    }

    #[test]
    fn parse_insert_in_any_position() {
        let cmd = list_cmd();
        let parsed = ParsedId {
            base: "ws.specs".into(),
            insert: false,
            spec: None,
        };

        let req = parse(&cmd, &parsed, Some("foo insert bar")).unwrap();
        assert!(req.insert);
        assert_eq!(req.rest, vec!["foo", "bar"]);
    }

    #[test]
    fn parse_spec_positional_and_pass_through() {
        let cmd = spec_cmd();
        let parsed = ParsedId {
            base: "ws.current".into(),
            insert: false,
            spec: None,
        };

        // Positional::Spec consumes first non-insert token as spec
        let req = parse(&cmd, &parsed, Some("demo extra")).unwrap();
        assert_eq!(req.spec.as_deref(), Some("demo"));
        assert_eq!(req.rest, vec!["extra"]);

        // Positional::PassThrough keeps first token in rest
        let pt_cmd = pass_through_cmd();
        let pt_parsed = ParsedId {
            base: "vocab.term".into(),
            insert: false,
            spec: Some("demo".into()),
        };
        let pt_req = parse(&pt_cmd, &pt_parsed, Some("first second")).unwrap();
        assert_eq!(pt_req.spec.as_deref(), Some("demo"));
        assert_eq!(pt_req.rest, vec!["first", "second"]);
    }

    #[test]
    fn parse_spec_agreement_and_conflict() {
        let cmd = spec_cmd();
        let parsed = ParsedId {
            base: "ws.current".into(),
            insert: false,
            spec: Some("demo".into()),
        };

        // Agreeing
        let req = parse(&cmd, &parsed, Some("demo")).unwrap();
        assert_eq!(req.spec.as_deref(), Some("demo"));

        // Conflicting
        let err = parse(&cmd, &parsed, Some("other")).unwrap_err();
        assert_eq!(
            err,
            "spec named twice: `demo` (command) and `other` (argument)"
        );
    }

    #[test]
    fn parse_invalid_spec_name() {
        let cmd = spec_cmd();
        let parsed = ParsedId {
            base: "ws.current".into(),
            insert: false,
            spec: None,
        };

        let err = parse(&cmd, &parsed, Some("foo/bar")).unwrap_err();
        assert_eq!(
            err,
            "invalid spec name `foo/bar`: use a-z, 0-9, '.', '-', '_' without path separators"
        );
    }

    #[test]
    fn parse_insert_not_supported_error() {
        let cmd = spec_cmd(); // insert_capable: false
        let parsed = ParsedId {
            base: "ws.current".into(),
            insert: false,
            spec: None,
        };

        let err = parse(&cmd, &parsed, Some("insert")).unwrap_err();
        assert_eq!(err, "`ws.current` does not support insertion");
    }
}
