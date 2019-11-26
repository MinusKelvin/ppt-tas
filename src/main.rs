use std::io::{ stdin, Read };
use nix::unistd::Pid;
use nix::sys::ptrace::{ attach, detach, cont, read, write, getregs, setregs, step };
use nix::sys::wait::{ waitpid, WaitPidFlag, WaitStatus };
use nix::sys::signal::Signal;

fn main() {
    let mut playback = false;
    for arg in std::env::args() {
        if arg == "playback" {
            playback = true;
        }
    }

    if playback {
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
}

fn breakpoint(pid: Pid, addr: u64) -> Result<(), nix::Error> {
    // set breakpoint
    let original_word = read(pid, addr as *mut _)?;
    write(pid, addr as *mut _, (original_word & !0xFF | 0xCC) as *mut _);

    // wait for PPT to hit the breakpoint
    cont(pid, None)?;
    wait_for_trap(pid)?;

    // replace original instruction
    write(pid, addr as *mut _, original_word as *mut _)?;

    // fix instructcion pointer
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

fn play(pid: Pid) -> Result<(), Box<dyn std::error::Error>> {
    // breakpoint for initial RNG
    breakpoint(pid, 0x14003F86B)?;

    let seed = match read_hex()? {
        Some(v) => v,
        None => return Ok(())
    };
    let mut regs = getregs(pid)?;
    // game expects rng to be in rax
    regs.rax = seed;
    setregs(pid, regs)?;

    // TODO: make the work by pressing start over (148 frames before timer starts)
    let mut skips = 164;
    loop {
        // breakpoint for input system
        breakpoint(pid, 0x1413C7D9A)?;

        if skips == 0 {
            let input = match read_hex()? {
                Some(v) => v,
                None => return Ok(())
            };

            let mut regs = getregs(pid)?;
            // game expects input bitfield to be in rbx
            regs.rbx = input;
            setregs(pid, regs)?;
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