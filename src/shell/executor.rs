// cash — Executor
//
// Unix-only. Runs processes via fork/exec/pipe/dup2.
// On non-Unix platforms this module compiles to stubs only —
// the real execution path is gated with #[cfg(unix)].

use super::pipeline::Pipeline;

#[derive(Debug)]
pub struct ExitStatus {
    pub code: i32,
}

impl ExitStatus {
    pub fn success(&self) -> bool { self.code == 0 }
}

// ---------------------------------------------------------------------------
// Unix implementation
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod unix {
    use std::ffi::CString;
    use std::os::unix::io::RawFd;
    use std::path::PathBuf;

    use nix::fcntl::{open, OFlag};
    use nix::sys::signal::{signal, SigHandler, Signal};
    use nix::sys::stat::Mode;
    use nix::sys::wait::{waitpid, WaitStatus};
    use nix::unistd::{close, dup2, execvpe, fork, pipe, setpgid, ForkResult, Pid};

    use super::super::pipeline::{Pipeline, RedirectKind, Stage};
    use super::ExitStatus;

    pub fn run(pipeline: &Pipeline) -> anyhow::Result<ExitStatus> {
        if pipeline.is_empty() {
            return Ok(ExitStatus { code: 0 });
        }
        if pipeline.stages.len() == 1 {
            run_single(&pipeline.stages[0])
        } else {
            run_pipeline_inner(&pipeline.stages)
        }
    }

    fn run_single(stage: &Stage) -> anyhow::Result<ExitStatus> {
        let bin  = resolve_binary(&stage.name)?;
        let argv = build_argv(stage);
        let envp = build_envp(stage);

        unsafe {
            signal(Signal::SIGINT,  SigHandler::SigIgn).ok();
            signal(Signal::SIGQUIT, SigHandler::SigIgn).ok();
        }

        match unsafe { fork() }? {
            ForkResult::Child => {
                let _ = setpgid(Pid::from_raw(0), Pid::from_raw(0));
                unsafe {
                    signal(Signal::SIGINT,  SigHandler::SigDfl).ok();
                    signal(Signal::SIGQUIT, SigHandler::SigDfl).ok();
                }
                if let Err(e) = apply_redirects(stage) {
                    eprintln!("cash: {}", e);
                    std::process::exit(1);
                }
                let err = execvpe(&bin, &argv, &envp).unwrap_err();
                match err {
                    nix::Error::ENOENT  => eprintln!("cash: {}: command not found", stage.name),
                    nix::Error::EACCES  => eprintln!("cash: {}: permission denied", stage.name),
                    nix::Error::ENOEXEC => eprintln!("cash: {}: not an executable", stage.name),
                    nix::Error::EISDIR  => eprintln!("cash: {}: is a directory", stage.name),
                    other               => eprintln!("cash: {}: {}", stage.name, other),
                }
                std::process::exit(126);
            }
            ForkResult::Parent { child } => {
                let status = wait_for(child);
                unsafe {
                    signal(Signal::SIGINT,  SigHandler::SigDfl).ok();
                    signal(Signal::SIGQUIT, SigHandler::SigDfl).ok();
                }
                Ok(status)
            }
        }
    }

    fn run_pipeline_inner(stages: &[Stage]) -> anyhow::Result<ExitStatus> {
        let n = stages.len();
        let mut pids: Vec<Pid> = Vec::with_capacity(n);

        let mut pipes: Vec<(RawFd, RawFd)> = Vec::with_capacity(n - 1);
        for _ in 0..n - 1 {
            pipes.push(pipe()?);
        }

        unsafe {
            signal(Signal::SIGINT,  SigHandler::SigIgn).ok();
            signal(Signal::SIGQUIT, SigHandler::SigIgn).ok();
        }

        for (i, stage) in stages.iter().enumerate() {
            let bin  = resolve_binary(&stage.name)
                .unwrap_or_else(|_| CString::new(stage.name.as_str()).unwrap());
            let argv = build_argv(stage);
            let envp = build_envp(stage);

            match unsafe { fork() }? {
                ForkResult::Child => {
                    let _ = setpgid(Pid::from_raw(0), Pid::from_raw(0));
                    unsafe {
                        signal(Signal::SIGINT,  SigHandler::SigDfl).ok();
                        signal(Signal::SIGQUIT, SigHandler::SigDfl).ok();
                    }
                    if i > 0     { dup2(pipes[i-1].0, 0).ok(); }
                    if i < n - 1 { dup2(pipes[i].1,   1).ok(); }
                    for &(r, w) in &pipes { close(r).ok(); close(w).ok(); }
                    if let Err(e) = apply_redirects(stage) {
                        eprintln!("cash: {}", e);
                        std::process::exit(1);
                    }
                    let _ = execvpe(&bin, &argv, &envp);
                    eprintln!("cash: {}: command not found", stage.name);
                    std::process::exit(127);
                }
                ForkResult::Parent { child } => { pids.push(child); }
            }
        }

        for (r, w) in pipes { close(r).ok(); close(w).ok(); }

        let mut last = ExitStatus { code: 0 };
        for (i, &pid) in pids.iter().enumerate() {
            let s = wait_for(pid);
            if i == pids.len() - 1 { last = s; }
        }

        unsafe {
            signal(Signal::SIGINT,  SigHandler::SigDfl).ok();
            signal(Signal::SIGQUIT, SigHandler::SigDfl).ok();
        }
        Ok(last)
    }

    fn wait_for(pid: Pid) -> ExitStatus {
        loop {
            match waitpid(pid, None) {
                Ok(WaitStatus::Exited(_, code))      => return ExitStatus { code },
                Ok(WaitStatus::Signaled(_, sig, _))  => return ExitStatus { code: 128 + sig as i32 },
                Ok(_) => continue,
                Err(_) => return ExitStatus { code: 1 },
            }
        }
    }

    fn apply_redirects(stage: &Stage) -> Result<(), String> {
        for redir in &stage.redirects {
            match redir.kind {
                RedirectKind::Stdin => {
                    let fd = open(redir.target.as_str(), OFlag::O_RDONLY, Mode::empty())
                        .map_err(|e| format!("{}: {}", redir.target, e))?;
                    dup2(fd, 0).map_err(|e| e.to_string())?;
                    close(fd).ok();
                }
                RedirectKind::Stdout => {
                    let fd = open(redir.target.as_str(),
                        OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC,
                        Mode::from_bits_truncate(0o644))
                        .map_err(|e| format!("{}: {}", redir.target, e))?;
                    dup2(fd, 1).map_err(|e| e.to_string())?;
                    close(fd).ok();
                }
                RedirectKind::Append => {
                    let fd = open(redir.target.as_str(),
                        OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_APPEND,
                        Mode::from_bits_truncate(0o644))
                        .map_err(|e| format!("{}: {}", redir.target, e))?;
                    dup2(fd, 1).map_err(|e| e.to_string())?;
                    close(fd).ok();
                }
                RedirectKind::Stderr => {
                    let fd = open(redir.target.as_str(),
                        OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC,
                        Mode::from_bits_truncate(0o644))
                        .map_err(|e| format!("{}: {}", redir.target, e))?;
                    dup2(fd, 2).map_err(|e| e.to_string())?;
                    close(fd).ok();
                }
            }
        }
        Ok(())
    }

    fn resolve_binary(name: &str) -> anyhow::Result<CString> {
        if name.contains('/') {
            return Ok(CString::new(name)?);
        }
        let path_var = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into());
        for dir in path_var.split(':') {
            let candidate: PathBuf = [dir, name].iter().collect();
            if candidate.exists() {
                return Ok(CString::new(candidate.to_str().unwrap())?);
            }
        }
        Ok(CString::new(name)?)
    }

    fn build_argv(stage: &Stage) -> Vec<CString> {
        let mut v = vec![CString::new(stage.name.as_str()).unwrap_or_default()];
        for a in &stage.args {
            v.push(CString::new(a.as_str()).unwrap_or_default());
        }
        v
    }

    fn build_envp(stage: &Stage) -> Vec<CString> {
        let mut env: std::collections::HashMap<String, String> =
            std::env::vars().collect();
        for (k, v) in &stage.env {
            env.insert(k.clone(), v.clone());
        }
        env.iter()
            .filter_map(|(k, v)| CString::new(format!("{}={}", k, v)).ok())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Public API — delegates to unix::run on Unix, stub on Windows
// ---------------------------------------------------------------------------

#[cfg(unix)]
pub fn run(pipeline: &Pipeline) -> anyhow::Result<ExitStatus> {
    unix::run(pipeline)
}

#[cfg(not(unix))]
pub fn run(pipeline: &Pipeline) -> anyhow::Result<ExitStatus> {
    eprintln!(
        "cash: executor not available on this platform (requires Unix/WSL)"
    );
    eprintln!(
        "cash: command '{}' cannot be run — use WSL or Linux",
        pipeline.stages.first().map(|s| s.name.as_str()).unwrap_or("?")
    );
    Ok(ExitStatus { code: 1 })
}
