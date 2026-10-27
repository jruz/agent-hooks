use log.nu

def run-sandbox [args: list<string>, env_vars: record]: nothing -> record<stdout: string, stderr: string, exit_code: int> {
    with-env $env_vars {
        "" | run-external sandbox ...$args | complete
    }
}

def describe [outcome: record]: nothing -> string {
    let stdout = $outcome.stdout | to json --raw
    let stderr = $outcome.stderr | to json --raw
    $"exit ($outcome.exit_code), stdout ($stdout), stderr ($stderr)"
}

def to-result [label: string, pass: bool]: record -> record<pass: bool, msg: string> {
    let outcome = $in
    let detail = if $pass { "passed" } else { describe $outcome }
    {pass: $pass, msg: $"($label) — ($detail)"}
}

def expect-stdout [label: string, expected: string]: record -> record<pass: bool, msg: string> {
    let outcome = $in
    let pass = $outcome.exit_code == 0 and $outcome.stdout == $expected
    $outcome | to-result $label $pass
}

def expect-failure [label: string, diagnostic: string]: record -> record<pass: bool, msg: string> {
    let outcome = $in
    let pass = $outcome.exit_code != 0 and ($outcome.stderr | str contains $diagnostic)
    $outcome | to-result $label $pass
}

def with-executable [name: path, use: closure]: string -> record<pass: bool, msg: string> {
    let body = $in
    let dir = ^mktemp --directory --tmpdir=. sandbox-args-check.XXXXXXXXXX | path expand
    let path = $dir | path join $name
    $body | save $path
    ^chmod u+x $path
    let result = do $use $path
    rm --recursive $dir
    $result
}

def check-argument-boundaries []: nothing -> record<pass: bool, msg: string> {
    let args = [
        "printf"
        "<%s>\n"
        "hello world"
        ""
        "*.nix"
        "*"
        "?"
        "[ab]"
        'quote "with" spaces'
    ]
    let expected = "<hello world>\n<>\n<*.nix>\n<*>\n<?>\n<[ab]>\n<quote \"with\" spaces>\n"
    run-sandbox $args {} | expect-stdout "preserves argument boundaries" $expected
}

def check-executable-path-with-spaces []: nothing -> record<pass: bool, msg: string> {
    "#!/bin/sh\nexec printf \"<%s>\\n\" \"$@\"\n" | with-executable "command with spaces" {|path|
        run-sandbox [$path "hello world" "" "*.nix"] {}
        | expect-stdout "runs executable paths with spaces" "<hello world>\n<>\n<*.nix>\n"
    }
}

def check-missing-command []: nothing -> record<pass: bool, msg: string> {
    run-sandbox [] {} | expect-failure "rejects a missing command" "[OPTIONS...] [--] COMMAND [ARGS...]"
}

def check-empty-command []: nothing -> record<pass: bool, msg: string> {
    run-sandbox [""] {} | expect-failure "rejects an empty command" "execvp :"
}

def check-command-is-not-bubblewrap-option []: nothing -> record<pass: bool, msg: string> {
    run-sandbox ["--help"] {} | expect-failure "does not treat the command as a bubblewrap option" "execvp --help:"
}

def main [] {
    let results = [
        (check-argument-boundaries)
        (check-executable-path-with-spaces)
        (check-missing-command)
        (check-empty-command)
        (check-command-is-not-bubblewrap-option)
    ]
    for result in $results {
        if $result.pass { log ok $result.msg } else { log err $result.msg }
    }
    if ($results | any {|result| not $result.pass }) {
        exit 1
    }
}
