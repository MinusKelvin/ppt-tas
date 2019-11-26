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

        let pid = pid.unwrap_or_else(|| { println!("No such process"); std::process::exit(0) });
        println!("{}", pid);
        attach(pid).unwrap();
        waitpid(pid, None).unwrap();
        if let Err(e) = play(pid) {
            eprintln!("An error occured: {}", e)
        }
        detach(pid).unwrap();
    }
}

fn play(pid: Pid) -> Result<(), Box<dyn std::error::Error>> {
    let original_word = read(pid, 0x1413C7D9A as *mut _)?;
    let mut skips = 164;

    loop {
        // set breakpoint
        write(pid, 0x1413C7D9A as *mut _, (original_word & !0xFF | 0xCC) as _)?;
        cont(pid, None)?;

        loop {
            match waitpid(pid, Some(WaitPidFlag::WSTOPPED))? {
                WaitStatus::Stopped(_, Signal::SIGTRAP) => break,
                WaitStatus::Stopped(_, signal) => cont(pid, signal)?,
                unknown => eprintln!("Something happened, but we don't know what: {:#?}", unknown)
            }
        }

        let mut reg = getregs(pid)?;
        // reset instruction pointer to correct address
        reg.rip = 0x1413C7D9A;

        if skips == 0 {
            let mut line = String::new();
            stdin().read_line(&mut line)?;
            let line = line.trim();
            if line.is_empty() {
                // remove breakpoint
                write(pid, 0x1413C7D9A as *mut _, original_word as *mut _)?;
                setregs(pid, reg)?;
                break
            }
            let input = u64::from_str_radix(line, 16)?;

            reg.rbx = input;
        } else {
            skips -= 1;
        }
        setregs(pid, reg)?;

        // advance past breakpoint
        write(pid, 0x1413C7D9A as *mut _, original_word as *mut _)?;
        step(pid, None)?;

        loop {
            match waitpid(pid, Some(WaitPidFlag::WSTOPPED))? {
                WaitStatus::Stopped(_, Signal::SIGTRAP) => break,
                WaitStatus::Stopped(_, signal) => cont(pid, signal)?,
                unknown => eprintln!("Something happened, but we don't know what: {:#?}", unknown)
            }
        }
    }
    Ok(())
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