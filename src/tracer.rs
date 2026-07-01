use crate::error::{Result, SysriftError};
use nix::libc::user_regs_struct;
use nix::sys::ptrace;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, Pid};
use std::ffi::CString;

pub fn set_regs(pid: Pid, regs: &user_regs_struct) -> Result<()> {
    ptrace::setregs(pid, *regs).map_err(SysriftError::Ptrace)
}

pub fn get_regs(pid: Pid) -> Result<user_regs_struct> {
    ptrace::getregs(pid).map_err(SysriftError::Ptrace)
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

            let mut is_entry = true;

            loop {
                ptrace::syscall(child, None).map_err(SysriftError::Ptrace)?;

                match waitpid(child, None).map_err(SysriftError::Ptrace)? {
                    WaitStatus::Exited(_, code) => {
                        println!("[tracer] child exited with code {}", code);
                        break;
                    }
                    WaitStatus::Signaled(_, sig, _) => {
                        println!("[tracer] child killed by signal {:?}", sig);
                        break;
                    }
                    WaitStatus::Stopped(_, Signal::SIGTRAP) => {
                        on_syscall_stop(child, is_entry)?;
                        is_entry = !is_entry;
                    }
                    WaitStatus::Stopped(_, sig) => {
                        println!("[tracer] child received signal {:?}, killing", sig);
                        let _ = nix::sys::signal::kill(child, Signal::SIGKILL);
                        break;
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