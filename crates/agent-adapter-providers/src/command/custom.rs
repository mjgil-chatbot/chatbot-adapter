use super::*;

pub(super) fn build(
    name: &str,
    policy: ApprovalPolicy,
    config: &ProviderConfig,
    request: &ChatRequest,
) -> Result<CommandSpec, BuildError> {
    let exe = config.exe.clone();
    let mut args = config.args.clone();

    match policy {
        ApprovalPolicy::DangerousDefault => {
            if config.unsafe_args.is_empty() {
                return Err(BuildError {
                    message: format!(
                        "Custom provider {} requires unsafe_args for dangerous mode",
                        name
                    ),
                });
            }
            args.extend(config.unsafe_args.clone());
        }
        ApprovalPolicy::Safe => {
            if !config.safe_args.is_empty() {
                args.extend(config.safe_args.clone());
            }
        }
    }

    args = args
        .into_iter()
        .map(|arg| match arg.as_str() {
            "{prompt}" => request.prompt.clone(),
            "{cwd}" => request.cwd.to_string_lossy().to_string(),
            "{model}" => request.model.clone().unwrap_or_default(),
            "{session}" => request
                .session
                .as_ref()
                .map(|session| session.id.clone())
                .unwrap_or_default(),
            "{provider}" => match &request.provider {
                ProviderId::Custom(name) => name.clone(),
                other => format!("{:?}", other).to_lowercase(),
            },
            _ => arg,
        })
        .collect();

    let mut env = std::collections::HashMap::new();
    for key in &config.env_allowlist {
        if let Some(value) = request.env.get(key) {
            env.insert(key.clone(), value.clone());
        }
    }

    Ok(CommandSpec {
        exe,
        args,
        env,
        cwd: request.cwd.clone(),
        stdin_mode: StdinMode::None,
        prompt_file: None,
        temp_files: Vec::new(),
        timeout: request.timeout_ms.map(Duration::from_millis),
        process_group: ProcessGroupMode::KillTree,
        mapper_id: config.mapper.clone(),
        transport_kind: provider_kind_to_transport(&config.kind),
        unsafe_args: config.unsafe_args.clone(),
        safe_args: config.safe_args.clone(),
        cleanup_handles: Vec::new(),
        extras: std::collections::HashMap::new(),
    })
}
