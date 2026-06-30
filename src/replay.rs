use crate::syscall;
use crate::trace_log::TraceReader;
use crate::tracer;

pub fn run(program: &str, args: &[String]) {
    let mut events = TraceReader::open("trace.log");
    let mut neutralized = false;
    let mut saved_buf_ptr: u64 = 0;

    tracer::run_traced(program, args, |pid, is_entry| {
        let mut regs = tracer::get_regs(pid);

        if is_entry {
            neutralized = is_replayable(regs.orig_rax);
            if neutralized {
                saved_buf_ptr = regs.rsi; // buffer pointer, arg index 1
                regs.orig_rax = 39;
                tracer::set_regs(pid, &regs);
            }
        } else {
            if let Some(event) = events.next() {
                if neutralized {
                    regs.rax = event.ret as u64;
                    tracer::set_regs(pid, &regs);

                    if let Some(data) = &event.data {
                        if !data.is_empty() && matches!(event.num, 0 | 17) {
                            syscall::write_memory(pid, saved_buf_ptr, data);
                        }
                    }
                }
                println!("[replay] {} -> ret={}", event.name, event.ret);
            } else {
                println!("[replay] warning: trace exhausted, no event for this syscall");
            }
        }
    });

    println!("[replay] done");
}

fn is_replayable(num: u64) -> bool {
    matches!(num, 0 | 17) // read, write, open, close, openat
}