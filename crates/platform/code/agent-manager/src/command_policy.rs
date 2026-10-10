use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandIntent {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandIntent {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookDisposition {
    Unverified,
    NoHooksDetected,
    ManagedIsolatedHooks,
    DeveloperHooksPresent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookDiscovery {
    pub core_hooks_path: Option<String>,
    pub pre_commit_present: bool,
    pub commit_msg_present: bool,
}

impl HookDiscovery {
    pub fn assess(&self, managed_isolated_path: Option<&str>) -> HookDisposition {
        if let Some(path) = self.core_hooks_path.as_deref() {
            if managed_isolated_path.is_some_and(|managed| managed == path) {
                return HookDisposition::ManagedIsolatedHooks;
            }
            return HookDisposition::DeveloperHooksPresent;
        }
        if self.pre_commit_present || self.commit_msg_present {
            HookDisposition::DeveloperHooksPresent
        } else {
            HookDisposition::NoHooksDetected
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    pub hooks: HookDisposition,
    pub audit_reason: Option<String>,
    pub remote_ci_required: bool,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            hooks: HookDisposition::Unverified,
            audit_reason: None,
            remote_ci_required: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandClass {
    LightweightInspection,
    ManagedGitWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPermit {
    pub class: CommandClass,
    pub requires_remote_ci: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    LocalBuildOrTestForbidden,
    ArbitraryExecutionChannelForbidden,
    UnknownProgram,
    GitSubcommandForbidden,
    DangerousGitOption,
    AuditRequired,
    HookPolicyUnsafe,
    RemoteCiRequired,
}

const BUILD_OR_TEST_PROGRAMS: &[&str] = &[
    "cargo",
    "rustc",
    "rustdoc",
    "clippy-driver",
    "nextest",
    "cargo-nextest",
    "make",
    "cmake",
    "ninja",
    "msbuild",
    "gcc",
    "g++",
    "cc",
    "c++",
    "clang",
    "clang++",
    "go",
    "javac",
    "pytest",
    "jest",
    "vitest",
];

const ARBITRARY_EXECUTION_PROGRAMS: &[&str] = &[
    "sh",
    "bash",
    "dash",
    "zsh",
    "fish",
    "cmd",
    "powershell",
    "pwsh",
    "python",
    "python3",
    "node",
    "deno",
    "bun",
    "perl",
    "ruby",
    "php",
    "npm",
    "pnpm",
    "yarn",
];

fn normalized_program(program: &str) -> String {
    let file_name = Path::new(program)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(program)
        .to_ascii_lowercase();
    file_name
        .strip_suffix(".exe")
        .unwrap_or(&file_name)
        .to_owned()
}

fn audit_present(context: &ExecutionContext) -> bool {
    context
        .audit_reason
        .as_deref()
        .is_some_and(|reason| !reason.trim().is_empty())
}

fn git_inspection(args: &[String]) -> Result<CommandPermit, DenyReason> {
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--ext-diff" | "--textconv"))
    {
        return Err(DenyReason::DangerousGitOption);
    }
    Ok(CommandPermit {
        class: CommandClass::LightweightInspection,
        requires_remote_ci: false,
    })
}

fn managed_write(context: &ExecutionContext) -> Result<CommandPermit, DenyReason> {
    if !audit_present(context) {
        return Err(DenyReason::AuditRequired);
    }
    if !context.remote_ci_required {
        return Err(DenyReason::RemoteCiRequired);
    }
    Ok(CommandPermit {
        class: CommandClass::ManagedGitWrite,
        requires_remote_ci: true,
    })
}

pub fn evaluate(
    intent: &CommandIntent,
    context: &ExecutionContext,
) -> Result<CommandPermit, DenyReason> {
    let program = normalized_program(intent.program.trim());

    if BUILD_OR_TEST_PROGRAMS.contains(&program.as_str()) {
        return Err(DenyReason::LocalBuildOrTestForbidden);
    }
    if ARBITRARY_EXECUTION_PROGRAMS.contains(&program.as_str()) {
        return Err(DenyReason::ArbitraryExecutionChannelForbidden);
    }
    if program != "git" {
        return Err(DenyReason::UnknownProgram);
    }

    let Some(subcommand) = intent.args.first().map(String::as_str) else {
        return Err(DenyReason::GitSubcommandForbidden);
    };
    if subcommand.starts_with('-') {
        return Err(DenyReason::GitSubcommandForbidden);
    }

    match subcommand {
        "status" | "diff" => git_inspection(&intent.args[1..]),
        "add" => managed_write(context),
        "commit" => {
            if !matches!(
                context.hooks,
                HookDisposition::NoHooksDetected | HookDisposition::ManagedIsolatedHooks
            ) {
                return Err(DenyReason::HookPolicyUnsafe);
            }
            managed_write(context)
        }
        "push" => managed_write(context),
        _ => Err(DenyReason::GitSubcommandForbidden),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audited_context() -> ExecutionContext {
        ExecutionContext {
            hooks: HookDisposition::ManagedIsolatedHooks,
            audit_reason: Some("managed BAM commit; GitHub CI is authoritative".into()),
            remote_ci_required: true,
        }
    }

    #[test]
    fn direct_and_wrapped_execution_channels_are_denied() {
        for program in ["cargo", "rustc", "cargo.exe"] {
            assert_eq!(
                evaluate(
                    &CommandIntent::new(program, ["test"]),
                    &ExecutionContext::default()
                ),
                Err(DenyReason::LocalBuildOrTestForbidden)
            );
        }

        for program in ["bash", "pwsh.exe", "python3", "node"] {
            assert_eq!(
                evaluate(
                    &CommandIntent::new(program, ["-c", "cargo test"]),
                    &ExecutionContext::default()
                ),
                Err(DenyReason::ArbitraryExecutionChannelForbidden)
            );
        }
    }

    #[test]
    fn lightweight_git_inspection_is_allowed_but_external_diff_is_not() {
        let permit = evaluate(
            &CommandIntent::new("git", ["diff", "--check"]),
            &ExecutionContext::default(),
        )
        .unwrap();
        assert_eq!(permit.class, CommandClass::LightweightInspection);
        assert!(!permit.requires_remote_ci);

        assert_eq!(
            evaluate(
                &CommandIntent::new("git", ["diff", "--ext-diff"]),
                &ExecutionContext::default()
            ),
            Err(DenyReason::DangerousGitOption)
        );
    }

    #[test]
    fn commit_requires_safe_hooks_and_audit() {
        let mut context = audited_context();
        context.hooks = HookDisposition::DeveloperHooksPresent;
        assert_eq!(
            evaluate(&CommandIntent::new("git", ["commit"]), &context),
            Err(DenyReason::HookPolicyUnsafe)
        );

        let mut context = audited_context();
        context.audit_reason = None;
        assert_eq!(
            evaluate(&CommandIntent::new("git", ["commit"]), &context),
            Err(DenyReason::AuditRequired)
        );

        assert!(evaluate(&CommandIntent::new("git", ["commit"]), &audited_context()).is_ok());
    }

    #[test]
    fn git_global_options_and_external_subcommands_fail_closed() {
        assert_eq!(
            evaluate(
                &CommandIntent::new("git", ["-c", "alias.run=!cargo test", "run"]),
                &audited_context()
            ),
            Err(DenyReason::GitSubcommandForbidden)
        );
        assert_eq!(
            evaluate(
                &CommandIntent::new("git", ["upload-pack"]),
                &audited_context()
            ),
            Err(DenyReason::GitSubcommandForbidden)
        );
    }

    #[test]
    fn hook_discovery_never_silently_treats_developer_hooks_as_safe() {
        let discovery = HookDiscovery {
            core_hooks_path: Some(".githooks".into()),
            pre_commit_present: true,
            commit_msg_present: true,
        };
        assert_eq!(
            discovery.assess(Some(".bam/hooks")),
            HookDisposition::DeveloperHooksPresent
        );
        assert_eq!(
            HookDiscovery {
                core_hooks_path: Some(".bam/hooks".into()),
                pre_commit_present: false,
                commit_msg_present: false,
            }
            .assess(Some(".bam/hooks")),
            HookDisposition::ManagedIsolatedHooks
        );
    }
}
