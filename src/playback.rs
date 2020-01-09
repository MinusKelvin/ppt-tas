use std::io::{ stdin, Read };
use nix::unistd::Pid;
use nix::sys::ptrace::{ attach, detach, cont, read, write, getregs, setregs, step };
use nix::sys::wait::{ waitpid, WaitPidFlag, WaitStatus };
use nix::sys::signal::Signal;

pub fn playback() {
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
    attach(pid).unwrap();
    waitpid(pid, None).unwrap();
    if let Err(e) = play(pid) {
        eprintln!("An error occured: {}", e)
    }
    detach(pid).unwrap();
}

fn play(pid: Pid) -> Result<(), Box<dyn std::error::Error>> {
    // breakpoint for initial RNG
    breakpoint(pid, 0x14003F87F)?;

    let seed = match read_hex()? {
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
                let (i, r) = match read_input()? {
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

fn read_hex() -> Result<Option<u64>, Box<dyn std::error::Error>> {
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    let line = line.trim();
    if line.is_empty() {
        return Ok(None)
    }
    Ok(Some(u64::from_str_radix(line, 16)?))
}

fn read_input() -> Result<Option<(u64, u64)>, Box<dyn std::error::Error>> {
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    if line.is_empty() {
        return Ok(None)
    }
    let mut input = 0;
    let mut repeat_index = None;
    let mut repeat_end = None;
    for (i, c) in line.char_indices() {
        match c.to_ascii_lowercase() {
            '<' if repeat_index.is_none() => input |= 0x01,
            '>' if repeat_index.is_none() => input |= 0x02,
            'd' if repeat_index.is_none() => input |= 0x04,
            'v' if repeat_index.is_none() => input |= 0x08,
            'l' if repeat_index.is_none() => input |= 0x10,
            'r' if repeat_index.is_none() => input |= 0x20,
            'h' if repeat_index.is_none() => input |= 0x40,
            '0'..='9' => if repeat_index.is_none() {
                repeat_index = Some(i);
            }
            ' ' => if repeat_index.is_some() && repeat_end.is_none() {
                repeat_end = Some(i);
            }
            _ => {}
        }
    }
    let repeat = match repeat_index {
        Some(start) => match repeat_end {
            Some(end) => u64::from_str_radix(&line[start..end], 10)?,
            None => u64::from_str_radix(&line[start..].trim(), 10)?
        }
        None => 1
    };
    Ok(Some((input, repeat)))
}

fn breakpoint(pid: Pid, addr: u64) -> Result<(), nix::Error> {
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

fn wait_for_trap(pid: Pid) -> Result<(), nix::Error> {
    loop {
        match waitpid(pid, Some(WaitPidFlag::WSTOPPED))? {
            WaitStatus::Stopped(_, Signal::SIGTRAP) => return Ok(()),
            WaitStatus::Stopped(_, signal) => cont(pid, signal)?,
            unknown => eprintln!("Something happened, but we don't know what: {:#?}", unknown)
        }
    }
}
