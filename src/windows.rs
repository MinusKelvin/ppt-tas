use crate::Result;
use winapi::um::psapi::{ EnumProcesses, EnumProcessModules, GetModuleBaseNameW };
use winapi::um::processthreadsapi::{ OpenProcess, OpenThread, GetThreadContext, SetThreadContext };
use winapi::um::winnt::*;
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::{ ReadProcessMemory, WriteProcessMemory };
use winapi::um::winbase::*;
use winapi::um::minwinbase::*;
use winapi::um::debugapi::*;
use std::os::windows::ffi::OsStringExt;

pub fn playback() -> Result<()> {
    let pid = find_ppt_process()?
        .unwrap_or_else(|| { println!("No such process"); std::process::exit(1) });
    
    unsafe {
        if DebugActiveProcess(pid) == 0 {
            panic!("could not debug ppt");
        }

        let mut dbg_event = Default::default();
        WaitForDebugEvent(&mut dbg_event, INFINITE);
        if dbg_event.dwDebugEventCode != CREATE_PROCESS_DEBUG_EVENT {
            panic!("first debug event should've been a CREATE_PROCESS_DEBUG_EVENT");
        }
        let process = dbg_event.u.CreateProcessInfo().hProcess;
        let mut tid = dbg_event.dwThreadId;
        let mut continue_kind = DBG_EXCEPTION_NOT_HANDLED;

        play(pid, &mut tid, &mut continue_kind, process)?;

        ContinueDebugEvent(pid, tid, continue_kind);

        if DebugActiveProcessStop(pid) == 0 {
            panic!("could not stop debugging ppt");
        }
    }
    
    Ok(())
}

fn play(pid: u32, tid: &mut u32, continue_kind: &mut u32, process: HANDLE) -> Result<()> {
    unsafe {
        // breakpoint for initial RNG
        let thread = breakpoint(pid, tid, continue_kind, process, 0x14003F87F)?;

        let seed = match crate::read_hex()? {
            Some(v) => v,
            None => return Ok(())
        };
        let mut regs = CONTEXT::default();
        regs.ContextFlags = CONTEXT_ALL;
        if GetThreadContext(thread, &mut regs) == 0 { panic!(); }
        // Game expects rng to be in rax
        // Game shifts down by 16 bits so the seed is only 16 bits large
        // Altering this line allows impossible seeds to be used (such as the one for the 19.71s
        // sprint TAS), which lets us see what could have been if sega hadn't clipped the seed
        // space.
        regs.Rax = seed & 0xFFFF;
        if SetThreadContext(thread, &regs) == 0 { panic!(); }
        CloseHandle(thread);

        // TODO: make the work by pressing start over (148 frames before timer starts)
        let mut skips = 164;
        let mut input = 0;
        let mut repeats = 0;
        loop {
            // breakpoint for input system
            let thread = breakpoint(pid, tid, continue_kind, process, 0x1413C7D9A)?;

            if skips == 0 {
                if repeats == 0 {
                    let (i, r) = match crate::read_input()? {
                        Some(v) => v,
                        None => return Ok(())
                    };
                    input = i;
                    repeats = r;
                }

                let mut regs = CONTEXT::default();
                regs.ContextFlags = CONTEXT_ALL;
                if GetThreadContext(thread, &mut regs) == 0 { panic!(); }
                // game expects input bitfield to be in rbx
                regs.Rbx = input;
                if SetThreadContext(thread, &regs) == 0 { panic!(); }
                repeats -= 1;
            } else {
                skips -= 1;
            }
            CloseHandle(thread);

            // advance past breakpoint
            step(pid, tid, continue_kind)?;
        }
    }
}

fn breakpoint(
    pid: u32, tid: &mut u32, continue_kind: &mut u32, process: HANDLE, address: u64
) -> Result<HANDLE> {
    unsafe {
        let mut original = 0u8;
        let mut rw = 0;
        if ReadProcessMemory(
            process, address as *mut _, &mut original as *mut _ as *mut _, 1, &mut rw
        ) == 0 {
            panic!("read failed");
        }

        if WriteProcessMemory(
            process, address as *mut _, &0xCC as *const _ as *const _, 1, &mut rw
        ) == 0 {
            panic!("write breakpoint failed");
        }

        loop {
            if ContinueDebugEvent(pid, *tid, *continue_kind) == 0 { panic!(); }
            let mut dbg_event = Default::default();
            if WaitForDebugEvent(&mut dbg_event, INFINITE) == 0 { panic!(); }
            *tid = dbg_event.dwThreadId;
            if dbg_event.dwDebugEventCode != EXCEPTION_DEBUG_EVENT {
                if dbg_event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT {
                    panic!("ppt exited");
                }
                *continue_kind = DBG_EXCEPTION_NOT_HANDLED;
                continue;
            }

            let info = &dbg_event.u.Exception().ExceptionRecord;
            if info.ExceptionCode != EXCEPTION_BREAKPOINT {
                *continue_kind = DBG_EXCEPTION_NOT_HANDLED;
                continue;
            }
            if info.ExceptionAddress as u64 != address {
                *continue_kind = DBG_EXCEPTION_NOT_HANDLED;
                continue;
            }

            if WriteProcessMemory(
                process, address as *mut _, &original as *const _ as *const _, 1, &mut rw
            ) == 0 {
                panic!("writeback failed");
            }

            let thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT, 0, *tid);
            let mut regs = CONTEXT::default();
            regs.ContextFlags = CONTEXT_ALL;
            if GetThreadContext(thread, &mut regs) == 0 { panic!(); }
            regs.Rip = address;
            if SetThreadContext(thread, &regs) == 0 { panic!(); }
            *continue_kind = DBG_CONTINUE;

            return Ok(thread);
        }
    }
}

fn step(pid: u32, tid: &mut u32, continue_kind: &mut u32) -> Result<()> {
    unsafe {
        let thread = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT, 0, *tid);
        let mut regs = CONTEXT::default();
        regs.ContextFlags = CONTEXT_ALL;
        if GetThreadContext(thread, &mut regs) == 0 { panic!(); }
        regs.EFlags |= 0x100;
        if SetThreadContext(thread, &regs) == 0 { panic!(); }
        CloseHandle(thread);

        loop {
            if ContinueDebugEvent(pid, *tid, *continue_kind) == 0 { panic!(); }
            let mut dbg_event = Default::default();
            if WaitForDebugEvent(&mut dbg_event, INFINITE) == 0 { panic!(); }
            *tid = dbg_event.dwThreadId;
            if dbg_event.dwDebugEventCode != EXCEPTION_DEBUG_EVENT {
                if dbg_event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT {
                    panic!("ppt exited");
                }
                *continue_kind = DBG_EXCEPTION_NOT_HANDLED;
                continue;
            }

            let info = &dbg_event.u.Exception().ExceptionRecord;
            if info.ExceptionCode != EXCEPTION_SINGLE_STEP {
                *continue_kind = DBG_EXCEPTION_NOT_HANDLED;
                continue;
            }
            *continue_kind = DBG_CONTINUE;

            return Ok(())
        }
    }
}

fn find_ppt_process() -> Result<Option<u32>> {
    unsafe {
        let mut pids = [0; 4096];
        let mut used = 0;
        if EnumProcesses(
            pids.as_mut_ptr(), std::mem::size_of_val(&pids) as u32, &mut used
        ) == 0 {
            panic!("failed to enumerate processes");
        }

        for &process in &pids[..used as usize/std::mem::size_of::<u32>()] {
            let handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, process
            );
            if !handle.is_null() {
                let mut module = 0 as *mut _;
                if EnumProcessModules(
                    handle,
                    &mut module,
                    std::mem::size_of::<*mut ()>() as u32,
                    &mut used
                ) != 0 {
                    let mut buffer = vec![0; 4096];
                    GetModuleBaseNameW(
                        handle, module, buffer.as_mut_ptr(), 2*buffer.len() as u32
                    );
                    for i in 0..buffer.len() {
                        if buffer[i] == 0 {
                            let s = std::ffi::OsString::from_wide(&buffer[..i]);
                            if let Some(s) = s.to_str() {
                                if s == "puyopuyotetris.exe" {
                                    CloseHandle(handle);
                                    return Ok(Some(process))
                                }
                            }
                            break
                        }
                    }
                }

                CloseHandle(handle);
            }
        }
        Ok(None)
    }
}
