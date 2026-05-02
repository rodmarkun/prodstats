use prodstats::install::{git_shim_script, shell_snippet};

#[test]
fn git_shim_logs_successful_pushes() {
    let script = git_shim_script("/usr/bin/git", "/usr/bin/prodstats");

    assert!(script.contains("PRODSTATS_GIT_WRAPPER_SKIP"));
    assert!(script.contains("\"$PRODSTATS\" log-git-command \"$rc\" \"$repo\" \"$@\""));
    assert!(script.contains("if [ \"${1:-}\" = \"push\" ]; then"));
}

#[test]
fn shell_snippet_marks_inner_git_call_so_shim_does_not_double_log() {
    let snippet = shell_snippet();

    assert!(snippet.contains("PRODSTATS_GIT_WRAPPER_SKIP=1 command git \"$@\""));
}
