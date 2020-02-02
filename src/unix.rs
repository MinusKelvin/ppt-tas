use crate::Result;
use nix::unistd::{ Pid, geteuid };
use nix::sys::ptrace::{ attach, detach, cont, read, write, getregs, setregs, step };
use nix::sys::wait::{ waitpid, WaitPidFlag, WaitStatus };
use nix::sys::signal::Signal;

pub fn playback() -> Result<()> {
    if !geteuid().is_root() {
        eprintln!("Need root permissions to playback a TAS.");
        std::process::exit(2);
    }

    let mut pid = None;
    for path in std::fs::read_dir("/proc").unwrap() {
        let name = path.unwrap().file_name();
        if let Some(raw_pid) = name.to_str().and_then(check) {
            pid = Some(raw_pid);
            break
        }
    }

    let pid = pid.unwrap_or_else(|| { println!("No such process"); std::process::exit(1) });
    println!("{}", pid);
    attach(pid)?;
    waitpid(pid, None)?;
    play(pid)?;
    detach(pid)
}

fn play(pid: Pid) -> Result<()> {
    // breakpoint for initial RNG
    breakpoint(pid, 0x14003F87F)?;

    let seed = match crate::read_hex()? {
        Some(v) => v,
        None => return Ok(())
    };
    let mut regs = getregs(pid)?;
    // game expects rng to be in rax
    // game shifts down by 16 bits so the seed is only 16 bits large
    // altering this line allows impossible seeds to be used (such as the one for the 19.96s sprint
    // TAS), which lets us see what could have been if sega hadn't clipped the seed space.
    regs.rax = seed & 0xFFFF;
    setregs(pid, regs)?;

    // TODO: make the work by pressing start over (148 frames before timer starts)
    let mut skips = 164;
    let mut input = 0;
    let mut repeats = 0;
    loop {
        // breakpoint for input system
        breakpoint(pid, 0x1413C7D9A)?;

        if skips == 0 {
            if repeats == 0 {
                let (i, r) = match crate::read_input()? {
                    Some(v) => v,
                    None => return Ok(())
                };
                input = i;
                repeats = r;
            }

            let mut regs = getregs(pid)?;
            // game expects input bitfield to be in rbx
            regs.rbx = input;
            setregs(pid, regs)?;
            repeats -= 1;
        } else {
            skips -= 1;
        }

        // advance past breakpoint
        step(pid, None)?;
        wait_for_trap(pid)?;
    }
}

fn check(entry: &str) -> Option<Pid> {
    let pid = entry.parse().ok()?;

    let mut cmdline = String::new();
    std::fs::File::open(&format!("/proc/{}/cmdline", pid)).ok()?
        .read_to_string(&mut cmdline).unwrap();
    if cmdline.starts_with("Z:") && cmdline.contains("puyopuyotetris") {
        Some(Pid::from_raw(pid))
    } else {
        None
    }
}

fn breakpoint(pid: Pid, addr: u64) -> Result<()> {
    // set breakpoint
    let original_word = read(pid, addr as *mut _)?;
    write(pid, addr as *mut _, (original_word & !0xFF | 0xCC) as *mut _)?;

    // wait for PPT to hit the breakpoint
    cont(pid, None)?;
    wait_for_trap(pid)?;

    // replace original instruction
    write(pid, addr as *mut _, original_word as *mut _)?;

    // fix instruction pointer
    let mut regs = getregs(pid)?;
    regs.rip = addr;
    setregs(pid, regs)
}

fn wait_for_trap(pid: Pid) -> Result<()> {
    loop {
        match waitpid(pid, Some(WaitPidFlag::WSTOPPED))? {
            WaitStatus::Stopped(_, Signal::SIGTRAP) => return Ok(()),
            WaitStatus::Stopped(_, signal) => cont(pid, signal)?,
            unknown => eprintln!("Something happened, but we don't know what: {:#?}", unknown)
        }
    }
}
