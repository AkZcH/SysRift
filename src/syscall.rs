use nix::libc::user_regs_struct;
use nix::sys::uio::process_vm_writev;
use nix::sys::uio::{RemoteIoVec, process_vm_readv};
use nix::unistd::Pid;
use std::io::IoSlice;
use std::io::IoSliceMut;

#[derive(Debug)]
pub struct SyscallArgs {
    pub num: u64,
    pub args: [u64; 6],
    pub ret: u64,
}

impl SyscallArgs {
    pub fn from_regs(regs: &user_regs_struct) -> Self {
        SyscallArgs {
            num: regs.orig_rax,
            args: [regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9],
            ret: regs.rax,
        }
    }
}

pub fn write_memory(pid: Pid, addr: u64, data: &[u8]) {
    let remote = RemoteIoVec {
        base: addr as usize,
        len: data.len(),
    };
    let local = IoSlice::new(data);

    if let Err(e) = process_vm_writev(pid, &[local], &[remote]) {
        eprintln!("[warn] write_memory failed: {:?}", e);
    }
}

pub fn read_memory(pid: Pid, addr: u64, len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    let remote = RemoteIoVec {
        base: addr as usize,
        len,
    };
    let local = IoSliceMut::new(&mut buf);

    match process_vm_readv(pid, &mut [local], &[remote]) {
        Ok(_) => buf,
        Err(e) => {
            eprintln!("[warn] read_memory failed: {:?}", e);
            Vec::new()
        }
    }
}

pub fn syscall_name(num: u64) -> &'static str {
    match num {
        0 => "read",
        1 => "write",
        2 => "open",
        3 => "close",
        4 => "stat",
        5 => "fstat",
        9 => "mmap",
        10 => "mprotect",
        11 => "munmap",
        12 => "brk",
        17 => "pread64",
        21 => "access",
        56 => "clone", // used by fork() on Linux
        57 => "fork",
        58 => "vfork",
        59 => "execve",
        61 => "wait4", // wait() maps to wait4
        90 => "chmod",
        158 => "arch_prctl",
        202 => "futex",
        218 => "set_tid_address",
        231 => "exit_group",
        257 => "openat",
        262 => "newfstatat",
        267 => "readlinkat",
        273 => "set_robust_list",
        302 => "prlimit64",
        318 => "getrandom",
        334 => "rseq",
        _ => "unknown",
    }
}
