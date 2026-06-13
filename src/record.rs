use crate::syscall;
use crate::trace_log::{TraceEvent, TraceWriter};
use crate::tracer;

pub fn run(program: &str, args: &[String]) {
    let mut writer = TraceWriter::create("trace.log");

    tracer::run_traced(program, args, |pid, is_entry| {
        if is_entry {
            return;
        }

        let regs = tracer::get_regs(pid);
        let call = syscall::SyscallArgs::from_regs(&regs);
        let ret = call.ret as i64;

        // Capture buffer for write (and later read)
        let data = if (call.num == 1 || call.num == 0 || call.num == 17) && ret > 0 {
            Some(syscall::read_memory(pid, call.args[1], ret as usize))
        } else {
            None
        };

        let event = TraceEvent {
            num: call.num,
            name: syscall::syscall_name(call.num).to_string(),
            args: call.args,
            ret,
            data,
        };

        writer.write_event(&event);
    });

    println!("[record] trace written to trace.log");
}
