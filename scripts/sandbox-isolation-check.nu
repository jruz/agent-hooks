use log.nu

def allowed-paths [extra: list<string>] {
    let home = $env.HOME
    [
        "/nix"
        "/dev"
        "/proc"
        "/tmp"
        "/etc/resolv.conf"
        "/etc/ssl"
        "/etc/static"
        "/etc/nix"
        "/run/current-system/sw"
        "/usr/bin/env"
        "/bin/sh"
        "/var/run/docker.sock"
        ($home | path join ".bun")
        ($home | path join ".local/share/pnpm")
        ($home | path join ".cache/pnpm")
        ($home | path join ".cargo/registry")
        ($home | path join ".cargo/git")
        ($home | path join ".gitconfig")
        ($home | path join ".ssh/known_hosts")
        $env.PWD
    ] | append $extra
}

def blocked-paths [] {
    let home = $env.HOME
    [
        ($home | path join ".cargo/credentials.toml")
        ($home | path join ".cargo/credentials")
    ]
}

def is-ancestor [path: string]: list<string> -> bool {
    any {|candidate| $candidate | str starts-with $"($path)/" }
}

def unexpected-entries [dir: path, allowed: list<string>] {
    ls --all --full-paths $dir
    | each {|entry|
        if $entry.name in $allowed {
            []
        } else if ($allowed | is-ancestor $entry.name) {
            unexpected-entries $entry.name $allowed
        } else {
            [$entry.name]
        }
    }
    | flatten
}

def can-write [dir: string] {
    let probe = $dir | path join $".sandbox-write-probe-(random uuid)"
    let outcome = do { ^touch $probe } | complete
    if $outcome.exit_code == 0 { rm $probe }
    $outcome.exit_code == 0
}

def check-unexpected-entries [allowed: list<string>] {
    let unexpected = unexpected-entries "/" $allowed
    if ($unexpected | is-empty) {
        [
            {pass: true, msg: "filesystem — only allowed entries"}
        ]
    } else {
        $unexpected | each {|path| {pass: false, msg: $"($path) — not allowed"} }
    }
}

def check-blocked [path: string]: nothing -> record<pass: bool, msg: string> {
    let exists = $path | path exists
    let detail = if $exists { "readable" } else { "blocked" }
    {pass: (not $exists), msg: $"($path) — ($detail)"}
}

def check-writable [dir: string, expected: bool] {
    let writable = can-write $dir
    let detail = if $writable { "writable" } else { "read-only" }
    {pass: ($writable == $expected), msg: $"($dir) — ($detail)"}
}

def main [...extra_allowed: string] {
    let allowed = allowed-paths $extra_allowed
    let results = [
        ...(check-unexpected-entries $allowed)
        ...(blocked-paths | each {|path| check-blocked $path })
        (check-writable $env.PWD true)
        (check-writable "/tmp" true)
        (check-writable "/nix" false)
        (check-writable "/etc/ssl" false)
    ]
    for result in $results {
        if $result.pass { log ok $result.msg } else { log err $result.msg }
    }
    if ($results | any {|result| not $result.pass }) {
        exit 1
    }
}
