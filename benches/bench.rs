#![feature(test)]

extern crate test;
extern crate dwarf;
extern crate gimli;
extern crate dwarf_bench;

#[cfg(feature = "libdwarf")]
use dwarf_bench::libdwarf;

#[cfg(feature = "elfutils")]
use dwarf_bench::libdw;

use std::os::unix::io::AsRawFd;

fn test_path() -> std::ffi::OsString {
    //std::env::args_os().next().unwrap()
    std::env::var_os("BENCH_FILE").unwrap()
}

fn elf_load() -> dwarf::Sections<dwarf::AnyEndian> {
    dwarf::elf::load(test_path()).unwrap()
}

#[bench]
fn info_rust_dwarf(b: &mut test::Bencher) {
    let sections = elf_load();
    b.iter(|| {
        if cfg!(feature = "io") {
            let sections = elf_load();
            impl_info_rust_dwarf(&sections);
        } else {
            impl_info_rust_dwarf(&sections);
        }
    });
}

fn impl_info_rust_dwarf(sections: &dwarf::Sections<dwarf::AnyEndian>) {
    let mut units = sections.compilation_units();
    while let Some(unit) = units.next().unwrap() {
        let abbrev = sections.abbrev(&unit.common).unwrap();
        let mut entries = unit.entries(&abbrev);
        while let Some(entry) = entries.next().unwrap() {
            test::black_box(entry.tag);
            for attribute in &entry.attributes {
                test::black_box(attribute.at);
                test::black_box(&attribute.data);
            }
        }
    }
}

#[bench]
fn info_gimli(b: &mut test::Bencher) {
    let sections = elf_load();
    b.iter(|| {
        if cfg!(feature = "io") {
            let sections = elf_load();
            impl_info_gimli(&sections);
        } else {
            impl_info_gimli(&sections);
        }
    });
}

fn impl_info_gimli(sections: &dwarf::Sections<dwarf::AnyEndian>) {
    let debug_info = gimli::DebugInfo::<gimli::LittleEndian>::new(&sections.debug_info);
    let debug_abbrev = gimli::DebugAbbrev::<gimli::LittleEndian>::new(&sections.debug_abbrev);
    let mut units = debug_info.units();
    while let Some(unit) = units.next().unwrap() {
        let abbrevs = unit.abbreviations(debug_abbrev).unwrap();
        let mut cursor = unit.entries(&abbrevs);
        while cursor.next_dfs().unwrap().is_some() {
            let entry = cursor.current().unwrap();
            test::black_box(entry.tag());
            let mut attrs = entry.attrs();
            while let Some(attr) = attrs.next().unwrap() {
                test::black_box(attr.name());
                test::black_box(attr.value());
            }
        }
    }
}

#[cfg(feature = "libdwarf")]
const DW_DLV_NO_ENTRY: std::os::raw::c_int = -1;
#[cfg(feature = "libdwarf")]
const DW_DLV_OK: std::os::raw::c_int = 0;
//const DW_DLV_ERROR: std::os::raw::c_int = 1;

#[cfg(feature = "libdwarf")]
const DW_DLA_DIE: libdwarf::Dwarf_Unsigned = 0x08;
#[cfg(feature = "libdwarf")]
const DW_DLA_ATTR: libdwarf::Dwarf_Unsigned = 0x0a;
#[cfg(feature = "libdwarf")]
const DW_DLA_LIST: libdwarf::Dwarf_Unsigned = 0x0f;

#[cfg(feature = "libdwarf")]
#[bench]
fn info_libdwarf(b: &mut test::Bencher) {
    b.iter(|| {
        let null = std::ptr::null_mut::<std::os::raw::c_void>();
        let file = std::fs::File::open(test_path()).unwrap();

        let fd = file.as_raw_fd();
        let access = 0; // DW_DLC_READ
        let errhand = None;
        let errarg = null as libdwarf::Dwarf_Ptr;
        let mut dbg = null as libdwarf::Dwarf_Debug;
        let error = null as *mut libdwarf::Dwarf_Error;
        let res = unsafe {
            libdwarf::dwarf_init(fd, access, errhand, errarg, &mut dbg, error)
        };
        assert_eq!(res, DW_DLV_OK);

        loop {
            let mut cu_header_length = 0;
            let mut version_stamp = 0;
            let mut abbrev_offset = 0;
            let mut address_size = 0;
            let mut next_cu_header_offset = 0;
            let res = unsafe {
                libdwarf::dwarf_next_cu_header(
                    dbg,
                    &mut cu_header_length,
                    &mut version_stamp,
                    &mut abbrev_offset,
                    &mut address_size,
                    &mut next_cu_header_offset,
                    error)
            };
            if res == DW_DLV_NO_ENTRY {
                break;
            }
            assert_eq!(res, DW_DLV_OK);

            let mut cu_die = null as libdwarf::Dwarf_Die;
            let res = unsafe {
                libdwarf::dwarf_siblingof(dbg, null as libdwarf::Dwarf_Die, &mut cu_die, error)
            };
            assert_eq!(res, DW_DLV_OK);

            info_libdwarf_die(dbg, cu_die);
        }

        let res = unsafe {
            libdwarf::dwarf_finish(dbg, error)
        };
        assert_eq!(res, DW_DLV_OK);
    });
}

#[cfg(feature = "libdwarf")]
fn info_libdwarf_die(dbg: libdwarf::Dwarf_Debug, in_die: libdwarf::Dwarf_Die) {
    let null = std::ptr::null_mut::<std::os::raw::c_void>();
    let error = null as *mut libdwarf::Dwarf_Error;
    let mut cur_die = in_die;

    info_libdwarf_attr(dbg, in_die);

    loop {
        let mut child_die = null as libdwarf::Dwarf_Die;
        let res = unsafe {
            libdwarf::dwarf_child(cur_die, &mut child_die, error)
        };
        if res != DW_DLV_NO_ENTRY {
            assert_eq!(res, DW_DLV_OK);
            info_libdwarf_die(dbg, child_die);
        }

        let mut sib_die = null as libdwarf::Dwarf_Die;
        let res = unsafe {
            libdwarf::dwarf_siblingof(dbg, cur_die, &mut sib_die, error)
        };
        if res == DW_DLV_NO_ENTRY {
            break;
        }
        assert_eq!(res, DW_DLV_OK);

        unsafe {
            libdwarf::dwarf_dealloc(dbg, cur_die as *mut std::os::raw::c_void, DW_DLA_DIE);
        };
        cur_die = sib_die;

        info_libdwarf_attr(dbg, cur_die);
    }

    unsafe {
        libdwarf::dwarf_dealloc(dbg, cur_die as *mut std::os::raw::c_void, DW_DLA_DIE);
    };
}

#[cfg(feature = "libdwarf")]
fn info_libdwarf_attr(dbg: libdwarf::Dwarf_Debug, die: libdwarf::Dwarf_Die) {
    let null = std::ptr::null_mut::<std::os::raw::c_void>();
    let error = null as *mut libdwarf::Dwarf_Error;

    let mut atlist = null as *mut libdwarf::Dwarf_Attribute;
    let mut atcnt = 0;
    let res = unsafe {
        libdwarf::dwarf_attrlist(die, &mut atlist, &mut atcnt, error)
    };
    if res == DW_DLV_NO_ENTRY {
        return;
    }
    assert_eq!(res, DW_DLV_OK);

    let atslice = unsafe {
        std::slice::from_raw_parts(atlist, atcnt as usize)
    };

    // TODO: read the attribute values

    for attr in atslice {
        unsafe {
            libdwarf::dwarf_dealloc(dbg, *attr as *mut std::os::raw::c_void, DW_DLA_ATTR);
        };
    }

    unsafe {
        libdwarf::dwarf_dealloc(dbg, atlist as *mut std::os::raw::c_void, DW_DLA_LIST);
    };
}

#[cfg(feature = "elfutils")]
#[bench]
fn info_elfutils(b: &mut test::Bencher) {
    b.iter(|| {
        let null = std::ptr::null_mut::<std::os::raw::c_void>();
        let file = std::fs::File::open(test_path()).unwrap();
        let fd = file.as_raw_fd();
        let dwarf = unsafe {
            libdw::dwarf_begin(fd, libdw::Dwarf_Cmd::DWARF_C_READ)
        };
        assert!(dwarf != null as *mut libdw::Dwarf);

        let mut offset = 0;
        loop {
            let mut next_offset = 0;
            let mut header_size = 0;
            let mut abbrev_offset = 0;
            let mut address_size = 0;
            let mut offset_size = 0;
            let res = unsafe {
                libdw::dwarf_nextcu(
                    dwarf,
                    offset,
                    &mut next_offset,
                    &mut header_size,
                    &mut abbrev_offset,
                    &mut address_size,
                    &mut offset_size)
            };
            if res > 0 {
                break;
            }
            assert_eq!(res, 0);

            let offdie = offset + header_size as u64;
            let mut stack = Vec::new();
            let mut die;
            unsafe {
                die = std::mem::uninitialized();
                let res = libdw::dwarf_offdie(dwarf, offdie, &mut die);
                assert_eq!(res, &mut die as *mut _);
            };
            stack.push(die);

            loop {
                let res = unsafe {
                    libdw::dwarf_getattrs(&mut die, Some(info_elfutils_attr), null, 0)
                };
                assert_eq!(res, 1);

                let mut next_die;
                let res = unsafe {
                    next_die = std::mem::uninitialized();
                    libdw::dwarf_child(&mut die, &mut next_die)
                };
                assert!(res >= 0);

                if res > 0 {
                    // No child, so read sibling
                    loop {
                        let res = unsafe {
                            next_die = std::mem::uninitialized();
                            libdw::dwarf_siblingof(&mut die, &mut next_die)
                        };
                        assert!(res >= 0);

                        if res > 0 {
                            // No sibling, so pop parent
                            if stack.len() == 0 {
                                break;
                            }
                            die = stack.pop().unwrap();
                        } else {
                            // Sibling
                            die = next_die;
                            break;
                        }
                    }
                    if stack.len() == 0 {
                        break;
                    }
                } else {
                    // Child, so push parent
                    stack.push(die);
                    die = next_die;
                }
            }

            offset = next_offset;
        }
    });
}

#[cfg(feature = "elfutils")]
unsafe extern "C" fn info_elfutils_attr(_: *mut libdw::Dwarf_Attribute, _: *mut std::os::raw::c_void) -> i32{
    0
}

#[bench]
fn line_rust_dwarf(b: &mut test::Bencher) {
    let sections = dwarf::elf::load(test_path()).unwrap();
    b.iter(|| {
        let mut r = &*sections.debug_line;
        let line_program = dwarf::line::LineProgram::read(&mut r, 0, sections.endian, 8, &[], &[]).unwrap();
        let mut lines = line_program.lines();
        while let Some(line) = lines.next().unwrap() {
            test::black_box(line);
        }
    });
}

#[bench]
fn line_gimli(b: &mut test::Bencher) {
    let sections = dwarf::elf::load(test_path()).unwrap();
    b.iter(|| {
        let debug_line = gimli::DebugLine::<gimli::LittleEndian>::new(&sections.debug_line);
        let header = debug_line.header(gimli::DebugLineOffset(0), 8, None, None).unwrap();
        let mut rows = header.rows();
        while let Some(row) = rows.next_row().unwrap() {
            test::black_box(row);
        }
    });
}

#[cfg(feature = "libdwarf")]
#[bench]
fn line_libdwarf(b: &mut test::Bencher) {
    b.iter(|| {
        let null = std::ptr::null_mut::<std::os::raw::c_void>();
        let file = std::fs::File::open(test_path()).unwrap();

        let fd = file.as_raw_fd();
        let access = 0; // DW_DLC_READ
        let errhand = None;
        let errarg = null as libdwarf::Dwarf_Ptr;
        let mut dbg = null as libdwarf::Dwarf_Debug;
        let error = null as *mut libdwarf::Dwarf_Error;
        let res = unsafe {
            libdwarf::dwarf_init(fd, access, errhand, errarg, &mut dbg, error)
        };
        assert_eq!(res, DW_DLV_OK);

        loop {
            let mut cu_header_length = 0;
            let mut version_stamp = 0;
            let mut abbrev_offset = 0;
            let mut address_size = 0;
            let mut next_cu_header_offset = 0;
            let res = unsafe {
                libdwarf::dwarf_next_cu_header(
                    dbg,
                    &mut cu_header_length,
                    &mut version_stamp,
                    &mut abbrev_offset,
                    &mut address_size,
                    &mut next_cu_header_offset,
                    error)
            };
            if res == DW_DLV_NO_ENTRY {
                break;
            }
            assert_eq!(res, DW_DLV_OK);

            let mut cu_die = null as libdwarf::Dwarf_Die;
            let res = unsafe {
                libdwarf::dwarf_siblingof(dbg, null as libdwarf::Dwarf_Die, &mut cu_die, error)
            };
            assert_eq!(res, DW_DLV_OK);

            let mut linebuf = null as *mut libdwarf::Dwarf_Line;
            let mut linecount = 0;
            let res = unsafe {
                libdwarf::dwarf_srclines(cu_die, &mut linebuf, &mut linecount, error)
            };
            if res == DW_DLV_NO_ENTRY {
                continue;
            }
            assert_eq!(res, DW_DLV_OK);

            unsafe {
                libdwarf::dwarf_srclines_dealloc(dbg, linebuf, linecount);
            }
        }

        let res = unsafe {
            libdwarf::dwarf_finish(dbg, error)
        };
        assert_eq!(res, DW_DLV_OK);
    });
}
