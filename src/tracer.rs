use crate::error::{Result, SysriftError};
use nix::libc;
use nix::libc::user_regs_struct;
use nix::sys::ptrace;
use nix::sys::ptrace::Options;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, Pid};
use std::collections::HashMap;
use std::ffi::CString;

pub fn set_regs(pid: Pid, regs: &user_regs_struct) -> Result<()> {
    ptrace::setregs(pid, *regs).map_err(SysriftError::Ptrace)
}

pub fn get_regs(pid: Pid) -> Result<user_regs_struct> {
    ptrace::getregs(pid).map_err(SysriftError::Ptrace)
}

fn get_event_pid(pid: Pid) -> Result<Pid> {
    let mut msg: libc::c_ulong = 0;
    let ret = unsafe {
        libc::ptrace(
            libc::PTRACE_GETEVENTMSG as libc::c_uint,
            libc::pid_t::from(pid),
            0usize,
            &mut msg as *mut libc::c_ulong,
        )
    };
    if ret == -1 {
        return Err(SysriftError::Ptrace(nix::Error::last()));
    }
    Ok(Pid::from_raw(msg as libc::pid_t))
}

pub fn run_traced<F>(program: &str, args: &[String], mut on_syscall_stop: F) -> Result<()>
where
    F: FnMut(Pid, bool) -> Result<()>,
{
    match unsafe { fork() }? {
        ForkResult::Child => {
            ptrace::traceme().map_err(SysriftError::Ptrace)?;

            let prog_c = CString::new(program).unwrap();
            let mut argv: Vec<CString> = vec![prog_c.clone()];
            for a in args {
                argv.push(CString::new(a.as_str()).unwrap());
            }

            let err = execvp(&prog_c, &argv).unwrap_err();
            Err(SysriftError::ExecFailed(err))
        }
        ForkResult::Parent { child } => {
            waitpid(child, None).map_err(SysriftError::Ptrace)?;

            let opts = Options::PTRACE_O_TRACESYSGOOD
                | Options::PTRACE_O_TRACEFORK
                | Options::PTRACE_O_TRACEVFORK
                | Options::PTRACE_O_TRACECLONE
                | Options::PTRACE_O_EXITKILL;
            ptrace::setoptions(child, opts).map_err(SysriftError::Ptrace)?;

            let mut tracees: HashMap<Pid, bool> = HashMap::new();
            tracees.insert(child, true);

            // Resume only the initial child to kick off the loop
            ptrace::syscall(child, None).map_err(SysriftError::Ptrace)?;

            loop {
                match waitpid(None, None).map_err(SysriftError::Ptrace)? {
                    WaitStatus::Exited(pid, code) => {
                        println!("[tracer] pid {} exited with code {}", pid, code);
                        tracees.remove(&pid);
                        if tracees.is_empty() {
                            break;
                        }
                        // no resume — pid is gone
                    }
                    WaitStatus::Signaled(pid, sig, _) => {
                        println!("[tracer] pid {} killed by signal {:?}", pid, sig);
                        tracees.remove(&pid);
                        if tracees.is_empty() {
                            break;
                        }
                    }
                    WaitStatus::PtraceSyscall(pid) => {
                        let is_entry = *tracees.get(&pid).unwrap_or(&true);
                        on_syscall_stop(pid, is_entry)?;
                        tracees.insert(pid, !is_entry);
                        ptrace::syscall(pid, None).map_err(SysriftError::Ptrace)?;
                    }
                    WaitStatus::PtraceEvent(pid, _, event) => {
                        let new_child = get_event_pid(pid)?;
                        println!("[tracer] new tracee pid {} (event {})", new_child, event);
                        tracees.insert(new_child, true);
                        // resume both the parent (which just forked) and the new child
                        ptrace::syscall(pid, None).map_err(SysriftError::Ptrace)?;
                        ptrace::syscall(new_child, None).map_err(SysriftError::Ptrace)?;
                    }
                    WaitStatus::Stopped(pid, sig) => {
                        if sig == Signal::SIGCHLD || sig == Signal::SIGCONT {
                            // forward the signal so wait() in the tracee unblocks
                            ptrace::syscall(pid, sig).map_err(SysriftError::Ptrace)?;
                        } else {
                            println!(
                                "[tracer] pid {} received signal {:?}, killing",
                                pid, sig
                            );
                            let _ = nix::sys::signal::kill(pid, Signal::SIGKILL);
                            tracees.remove(&pid);
                            if tracees.is_empty() {
                                break;
                            }
                        }
                    }
                    other => {
                        println!("[tracer] unexpected status: {:?}", other);
                    }
                }
            }

            Ok(())
        }
    }
}