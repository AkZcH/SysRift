use nix::libc::user_regs_struct;
use nix::sys::ptrace;
use nix::sys::signal::Signal;
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{ForkResult, Pid, execvp, fork};
use std::ffi::CString;

pub fn set_regs(pid: Pid, regs: &nix::libc::user_regs_struct) {
    ptrace::setregs(pid, *regs).expect("setregs failed");
}

pub fn get_regs(pid: Pid) -> user_regs_struct {
    ptrace::getregs(pid).expect("getregs failed")
}

pub fn run_traced<F>(program: &str, args: &[String], mut on_syscall_stop: F)
where
    F: FnMut(Pid, bool), // (pid, is_entry)
{
    match unsafe { fork() }.expect("fork failed") {
        ForkResult::Child => {
            // Let the parent trace us
            ptrace::traceme().expect("traceme failed");

            // Build argv for execvp: program name + args, all as CStrings
            let prog_c = CString::new(program).unwrap();
            let mut argv: Vec<CString> = vec![prog_c.clone()];
            for a in args {
                argv.push(CString::new(a.as_str()).unwrap());
            }

            // Replace this process image with the target program.
            // This raises a SIGTRAP that the parent will see as the first wait().
            let err = execvp(&prog_c, &argv).unwrap_err();
            panic!("execvp failed: {:?}", err);
        }
        ForkResult::Parent { child } => {
            // Wait for the initial stop caused by execvp's SIGTRAP
            waitpid(child, None).expect("waitpid failed");

            // is_entry toggles each stop: true = entering a syscall,
            // false = exiting it. ptrace gives us no direct flag for this —
            // every syscall produces TWO stops (entry, then exit), so we
            // just alternate.
            let mut is_entry = true;

            loop {
                // Ask the kernel to run until the next syscall entry/exit
                ptrace::syscall(child, None).expect("ptrace syscall failed");

                match waitpid(child, None).expect("waitpid failed") {
                    WaitStatus::Exited(_, code) => {
                        println!("[tracer] child exited with code {}", code);
                        break;
                    }
                    WaitStatus::Signaled(_, sig, _) => {
                        println!("[tracer] child killed by signal {:?}", sig);
                        break;
                    }
                    WaitStatus::Stopped(_, Signal::SIGTRAP) => {
                        on_syscall_stop(child, is_entry);
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
        }
    }
}
