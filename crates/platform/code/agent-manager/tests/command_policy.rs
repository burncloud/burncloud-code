use bam::command_policy::{
    evaluate, CommandClass, CommandIntent, DenyReason, ExecutionContext, HookDisposition,
};

fn audited() -> ExecutionContext {
    ExecutionContext {
        hooks: HookDisposition::ManagedIsolatedHooks,
        audit_reason: Some("remote CI remains mandatory".into()),
        remote_ci_required: true,
    }
}

#[test]
fn local_quality_commands_are_never_permitted() {
    for program in ["cargo", "rustc", "clippy-driver", "nextest", "cmake"] {
        let decision = evaluate(&CommandIntent::new(program, ["--version"]), &audited());
        assert_eq!(decision, Err(DenyReason::LocalBuildOrTestForbidden));
    }
}

#[test]
fn shell_and_interpreter_wrappers_are_not_an_escape_hatch() {
    for program in ["sh", "bash", "cmd.exe", "powershell.exe", "python", "node"] {
        let decision = evaluate(
            &CommandIntent::new(program, ["-c", "cargo test"]),
            &audited(),
        );
        assert_eq!(
            decision,
            Err(DenyReason::ArbitraryExecutionChannelForbidden)
        );
    }
}

#[test]
fn push_is_audited_and_keeps_remote_ci_authoritative() {
    let permit = evaluate(&CommandIntent::new("git", ["push"]), &audited()).unwrap();
    assert_eq!(permit.class, CommandClass::ManagedGitWrite);
    assert!(permit.requires_remote_ci);

    let mut no_remote_ci = audited();
    no_remote_ci.remote_ci_required = false;
    assert_eq!(
        evaluate(&CommandIntent::new("git", ["push"]), &no_remote_ci),
        Err(DenyReason::RemoteCiRequired)
    );
}
