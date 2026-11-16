//! Launch result handling for `apply`: capacity errors exit 75 (#1137) and the
//! id is checked as a whole instance id before `launched <id>` is printed (#1136).

/// Shell run before `aws ec2 run-instances`: a private file for its stderr.
pub(super) const PRE: &str =
    "errf=\"$(mktemp)\" || { echo 'ERROR: cannot create a temp file' >&2; exit 1; }\n\
    trap 'rm -f \"$errf\"' EXIT\n\
    rc=0\n\
    newid=\"$(";

/// Tail of the `run-instances` command: id only, stderr captured, rc kept.
pub(super) const CAPTURE: &str =
    " --query 'Instances[0].InstanceId' --output text 2>\"$errf\")\" || rc=$?\n";

/// Shell run after it. The classifier reads only the error code inside
/// `An error occurred (<Code>)` and matches it whole; empty or unreadable
/// stderr yields no code, so it is never capacity.
pub(super) const POST: &str = "if [ \"$rc\" -ne 0 ]; then\n\
    cat \"$errf\" >&2\n\
    code=\"$(sed -n 's/^.*An error occurred (\\([A-Za-z0-9.]*\\)).*$/\\1/p' \"$errf\" | head -n 1)\" || code=''\n\
    case \"$code\" in InsufficientInstanceCapacity|SpotMaxPriceTooLow|MaxSpotInstanceCountExceeded|Unsupported) exit 75;; esac\n\
    exit \"$rc\"\n\
    fi\n\
    h=\"${newid#i-}\"\n\
    case \"$newid\" in i-*) ;; *) h='!';; esac\n\
    case \"$h\" in *[!0-9a-f]*) h='!';; esac\n\
    if [ \"${#h}\" -ne 8 ] && [ \"${#h}\" -ne 17 ]; then echo 'ERROR: run-instances returned something that is not an instance id' >&2; exit 1; fi\n\
    echo \"launched $newid\"\n";

/// Printed after `present <ids>` when a live owned box is adopted.
pub(super) const ADOPT: &str = "set -- $owned; echo \"adopted $1\"; exit 0\n";
