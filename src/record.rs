use crate::error::Result;
use crate::syscall;
use crate::trace_log::{TraceEvent, TraceWriter};
use crate::tracer;

pub fn run(program: &str, args: &[String]) -> Result<()> {
    let mut writer = TraceWriter::create("trace.log")?;

    tracer::run_traced(program, args, |pid, is_entry| {
        if is_entry {
            return Ok(());
        }

        let regs = tracer::get_regs(pid)?;
        let call = syscall::SyscallArgs::from_regs(&regs);
        let ret = call.ret as i64;

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

        writer.write_event(&event)?;
        Ok(())
    })?;

    println!("[record] trace written to trace.log");
    Ok(())
}