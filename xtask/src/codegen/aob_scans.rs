use std::env;
use std::ffi::c_void;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use heck::AsSnakeCase;
use log::*;
use rayon::prelude::*;
use textwrap::dedent;
use widestring::U16CString;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, BOOL, CHAR, DBG_CONTINUE};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
};
use windows::Win32::System::Diagnostics::Debug::{
    ContinueDebugEvent, ReadProcessMemory, WaitForDebugEventEx, DEBUG_EVENT,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32First, Module32Next, MODULEENTRY32, TH32CS_SNAPMODULE,
};
use windows::Win32::System::Threading::{CreateProcessW, OpenProcess, *};

const AOBS: &[(&'static str, &'static str)] = &[
    ("ChrDbgFlags", "?? 80 3D ?? ?? ?? ?? 00 0F 85 ?? ?? ?? ?? 32 C0 48"),
    ("CSFD4VirtualMemoryFlag", "48 8B 3D ?? ?? ?? ?? 48 85 FF 74 ?? 48 8B 49"),
    ("CSFlipper", "48 8B 0D ?? ?? ?? ?? 80 BB D7 00 00 00 00 0F 84 CE 00 00 00 48 85 C9 75 2E"),
    ("CSLuaEventManager", "48 8B 05 ?? ?? ?? ?? 48 85 C0 74 ?? 41 BE 01 00 00 00 44 89 74"),
    ("CSMenuMan", "E8 ?? ?? ?? ?? 4C 8B F8 48 85 C0 0F 84 ?? ?? ?? ?? 48 8B 0D"),
    ("CSMenuManImp", "48 8B 0D ?? ?? ?? ?? 48 8B 49 08 E8 ?? ?? ?? ?? 48 8B D0 48 8B CE E8 ?? ?? ?? ??"),
    ("CSNetMan", "48 8B 0D ?? ?? ?? ?? 48 85 C9 74 5E 48 8B 89 ?? ?? ?? ?? B2 01"),
    ("CSRegulationManager", "48 8B 0D ?? ?? ?? ?? 48 85 C9 74 0B 4C 8B C0 48 8B D7"),
    ("CSSessionManager", "48 8B 05 ?? ?? ?? ?? 48 89 9C 24 E8 00 00 00 48 89 B4 24 B0 00 00 00 4C 89 A4 24 A8 00 00 00 4C 89 AC 24 A0 00 00 00 48 85 C0"),
    ("DamageCtrl", "48 8B 05 ?? ?? ?? ?? 49 8B D9 49 8B F8 48 8B F2 48 85 C0 75 2E"),
    ("FieldArea", "48 8B 3D ?? ?? ?? ?? 48 85 FF 0F 84 ?? ?? ?? ?? 45 38 66 34"),
    ("GameDataMan", "48 8B 05 ?? ?? ?? ?? 48 85 C0 74 05 48 8B 40 58 C3 C3"),
    ("GameMan", "48 8B 15 ?? ?? ?? ?? 41 B0 01 48 8B 0D ?? ?? ?? ?? 48 81 C2 10 0E 00 00"),
    ("GroupMask", "?? 80 3D ?? ?? ?? ?? 00 0F 10 00 0F 11 45 D0 0F 84 ?? ?? ?? ?? 80 3D"),
    ("HitIns", "48 8B 05 ?? ?? ?? ?? 48 8D 4C 24 ?? 48 89 4c 24 ?? 0F 10 44 24 70"),
    ("MapItemMan", "48 8B 0D ?? ?? ?? ?? C7 44 24 50 FF FF FF FF C7 45 A0 FF FF FF FF 48 85 C9 75 2E"),
    ("MenuManIns", "48 8b 0d ?? ?? ?? ?? 48 8b 53 08 48 8b 92 d8 00 00 00 48 83 c4 20 5b"),
    ("MsgRepository", "48 8B 3D ?? ?? ?? ?? 44 0F B6 30 48 85 FF 75 26"),
    ("SoloParamRepository", "48 8B 0D ?? ?? ?? ?? 48 85 C9 0F 84 ?? ?? ?? ?? 45 33 C0 BA 8D 00 00 00 E8"),
    ("WorldChrMan", "48 8B 05 ?? ?? ?? ?? 48 85 C0 74 0F 48 39 88 ?? ?? ?? ?? 75 06 89 B1 5C 03 00 00 0F 28 05 ?? ?? ?? ?? 4C 8D 45 E7"),
    ("WorldChrManDbg", "48 8B 0D ?? ?? ?? ?? 89 5C 24 20 48 85 C9 74 12 B8 ?? ?? ?? ?? 8B D8"),
    ("WorldChrManImp", "48 8b 05 ?? ?? ?? ?? 48 89 98 70 84 01 00 4c 89 ab 74 06 00 00 4c 89 ab 7c 06 00 00 44 88 ab 84 06 00 00 41 83 7f 4c 00"),
];

pub struct Version(u32, u32, u32);

impl Version {
    fn to_fromsoft_string(&self) -> String {
        format!("{}.{:02}.{}", self.0, self.1, self.2)
    }
}

fn szcmp(source: &[CHAR], s: &str) -> bool {
    source.iter().zip(s.chars()).all(|(a, b)| a.0 == b as u8)
}

fn into_needle(pattern: &str) -> Vec<Option<u8>> {
    pattern
        .split(' ')
        .map(|byte| match byte {
            "?" | "??" => None,
            x => u8::from_str_radix(x, 16).ok(),
        })
        .collect::<Vec<_>>()
}

fn naive_search(bytes: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    bytes.windows(pattern.len()).position(|wnd| {
        wnd.iter()
            .zip(pattern.iter())
            .all(|(byte, pattern)| match pattern {
                Some(x) => byte == x,
                None => true,
            })
    })
}

fn read_base_module_data(proc_name: &str, pid: u32) -> Option<(usize, Vec<u8>)> {
    let module_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, pid) };
    let mut module_entry = MODULEENTRY32::default();
    module_entry.dwSize = std::mem::size_of::<MODULEENTRY32>() as _;

    unsafe { Module32First(module_snapshot, &mut module_entry) };

    loop {
        if szcmp(&module_entry.szModule, proc_name) {
            let process = unsafe { OpenProcess(PROCESS_ALL_ACCESS, true, pid) };
            let mut buf = vec![0u8; module_entry.modBaseSize as usize];
            let mut bytes_read = 0usize;
            unsafe {
                ReadProcessMemory(
                    process,
                    module_entry.modBaseAddr as *mut c_void,
                    buf.as_mut_ptr() as *mut c_void,
                    module_entry.modBaseSize as usize,
                    &mut bytes_read,
                )
            };
            println!(
                "Read {:x} out of {:x} bytes",
                bytes_read, module_entry.modBaseSize
            );
            unsafe { CloseHandle(process) };
            return Some((module_entry.modBaseAddr as usize, buf));
        }
        if !unsafe { Module32Next(module_snapshot, &mut module_entry).as_bool() } {
            break;
        }
    }
    None
}

fn get_base_module_bytes(exe_path: &PathBuf) -> Option<(usize, Vec<u8>)> {
    let mut process_info = PROCESS_INFORMATION::default();
    let mut startup_info = STARTUPINFOW::default();
    startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as _;

    let mut exe = U16CString::from_str(exe_path.to_str().unwrap())
        .unwrap()
        .into_vec();
    exe.push(0);

    let process = unsafe {
        CreateProcessW(
            PCWSTR(exe.as_ptr()),
            PWSTR(null_mut()),
            null(),
            null(),
            BOOL::from(false),
            DEBUG_PROCESS | DETACHED_PROCESS,
            null(),
            PCWSTR(null()),
            &mut startup_info,
            &mut process_info,
        )
    };

    if !process.as_bool() {
        eprintln!(
            "Could not create process: {:x}",
            unsafe { GetLastError() }.0
        );
        return None;
    }

    println!(
        "Process handle={:x} pid={}",
        process_info.hProcess.0, process_info.dwProcessId
    );

    let mut debug_event = DEBUG_EVENT::default();

    loop {
        unsafe { WaitForDebugEventEx(&mut debug_event, 1000) };
        unsafe {
            ContinueDebugEvent(
                process_info.dwProcessId,
                process_info.dwThreadId,
                DBG_CONTINUE.0 as _,
            )
        };
        if debug_event.dwDebugEventCode.0 == 2 {
            break;
        }
    }

    let ret = read_base_module_data(
        exe_path.file_name().unwrap().to_str().unwrap(),
        process_info.dwProcessId,
    );

    unsafe { TerminateProcess(process_info.hProcess, 0) };

    ret
}

fn find_aobs(bytes: Vec<u8>) -> Vec<(&'static str, usize)> {
    let mut aob_offsets = AOBS
        .into_par_iter()
        .filter_map(|(name, aob)| {
            if let Some(r) = naive_search(&bytes, &into_needle(aob)) {
                Some((name, r))
            } else {
                eprintln!("{name:24} not found");
                None
            }
        })
        .map(|offset| {
            (
                offset.0,
                offset.1,
                u32::from_le_bytes(bytes[offset.1 + 3..offset.1 + 7].try_into().unwrap()),
            )
        })
        .map(|offset| (*offset.0, (offset.2 + 7) as usize + offset.1))
        .collect::<Vec<_>>();

    aob_offsets.sort_by(|a, b| a.0.cmp(b.0));

    aob_offsets
}

fn get_file_version(file: &Path) -> Version {
    let mut file_path = file.to_string_lossy().to_string();
    file_path.push(0 as char);
    let file_path = widestring::U16CString::from_str(file_path).unwrap();
    let mut version_info_size =
        unsafe { GetFileVersionInfoSizeW(PCWSTR(file_path.as_ptr()), null_mut()) };
    let mut version_info_buf = vec![0u8; version_info_size as usize];
    unsafe {
        GetFileVersionInfoW(
            PCWSTR(file_path.as_ptr()),
            0,
            version_info_size,
            version_info_buf.as_mut_ptr() as _,
        )
    };

    let mut version_info: *mut VS_FIXEDFILEINFO = null_mut();
    unsafe {
        VerQueryValueW(
            version_info_buf.as_ptr() as _,
            PCWSTR(widestring::U16CString::from_str("\\\\\0").unwrap().as_ptr()),
            &mut version_info as *mut *mut _ as _,
            &mut version_info_size,
        )
    };
    let version_info = unsafe { version_info.as_ref().unwrap() };
    let major = (version_info.dwFileVersionMS >> 16) & 0xffff;
    let minor = (version_info.dwFileVersionMS) & 0xffff;
    let patch = (version_info.dwFileVersionLS >> 16) & 0xffff;

    Version(major, minor, patch)
}

fn codegen_struct() -> String {
    let mut generated = String::new();

    generated.extend("pub struct BaseAddresses {\n".chars());
    generated.extend(
        AOBS.into_iter()
            .map(|(name, _)| format!("    pub {}: usize,\n", AsSnakeCase(name)))
            .collect::<Vec<_>>()
            .join("")
            .chars(),
    );
    generated.extend("}\n\n".chars());
    generated.extend("impl BaseAddresses {\n".chars());
    generated.extend("    pub fn with_module_base_addr(self, base: usize) -> BaseAddresses {\n".chars());
    generated.extend("        BaseAddresses {\n".chars());
    generated.extend(
        AOBS.into_iter()
            .map(|(name, _)| format!("            {}: self.{} + base,\n", AsSnakeCase(name), AsSnakeCase(name)))
            .collect::<Vec<_>>()
            .join("")
            .chars(),
    );
    generated.extend("        }\n    }\n}\n\n".chars());
    generated
}

fn codegen_version(ver: &Version, aobs: &[(&str, usize)]) -> String {
    let mut string = aobs.into_iter().fold(
        format!(
            "pub const BASE_ADDRESSES_{}_{:02}_{}: BaseAddresses = BaseAddresses {{\n",
            ver.0, ver.1, ver.2
        ),
        |mut o, (name, offset)| {
            o.push_str(&format!("    {}: 0x{:x},\n", AsSnakeCase(name), offset));
            o
        },
    );
    string.push_str("};\n\n");
    string
}

fn patches_paths() -> impl Iterator<Item = PathBuf> {
    let base_path = PathBuf::from(
        env::var("ERPT_PATCHES_PATH").expect(&dedent(r#"
            ERPT_PATCHES_PATH environment variable undefined.
            Check the documentation: https://github.com/veeenu/eldenring-practice-tool/README.md#building
        "#)),
    );
    base_path
        .read_dir()
        .expect("Couldn't scan patches directory")
        .map(Result::unwrap)
        .map(|dir| dir.path().join("Game").join("eldenring.exe"))
}

fn codegen_base_addresses_path() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
        .join("lib")
        .join("libeldenring")
        .join("src")
        .join("base_addresses.rs")
}

pub(crate) fn get_base_addresses() {
    let codegen = patches_paths()
        .filter(|p| p.exists())
        .map(|exe| {
            let version = get_file_version(&exe);
            let exe = exe.canonicalize().unwrap();
            println!("\nVERSION {}: {:?}", version.to_fromsoft_string(), exe);

            let (_base_addr, bytes) = get_base_module_bytes(&exe).unwrap();
            let mem_aobs = find_aobs(bytes);
            let version_base_addrs = codegen_version(&version, &mem_aobs);
            version_base_addrs
        })
        .fold(codegen_struct(), |mut o, i| {
            o.extend(i.chars());
            o
        });

    File::create(codegen_base_addresses_path())
        .unwrap()
        .write_all(codegen.as_bytes())
        .unwrap();
}