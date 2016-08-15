#![feature(test)]

extern crate test;
extern crate dwarf;
extern crate gimli;
extern crate dwarf_bench;
use dwarf_bench::libdwarf;

use std::os::unix::io::AsRawFd;

#[bench]
fn info_rust_dwarf(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
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
    });
}

#[bench]
fn info_gimli(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let debug_info = gimli::DebugInfo::<gimli::LittleEndian>::new(&sections.debug_info);
        let debug_abbrev = gimli::DebugAbbrev::<gimli::LittleEndian>::new(&sections.debug_abbrev);
        for unit in debug_info.units() {
            let unit = unit.unwrap();
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
    });
}

const DW_DLV_NO_ENTRY: std::os::raw::c_int = -1;
const DW_DLV_OK: std::os::raw::c_int = 0;
//const DW_DLV_ERROR: std::os::raw::c_int = 1;

const DW_DLA_DIE: libdwarf::Dwarf_Unsigned = 0x08;
const DW_DLA_ATTR: libdwarf::Dwarf_Unsigned = 0x0a;
const DW_DLA_LIST: libdwarf::Dwarf_Unsigned = 0x0f;

#[bench]
fn info_libdwarf(b: &mut test::Bencher) {
    let null = std::ptr::null_mut::<std::os::raw::c_void>();
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let file = std::fs::File::open(path).unwrap();

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

    b.iter(|| {
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
    });

    let res = unsafe {
        libdwarf::dwarf_finish(dbg, error)
    };
    assert_eq!(res, DW_DLV_OK);
}

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

#[bench]
fn line_rust_dwarf(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let mut r = &*sections.debug_line;
        let line_program = dwarf::line::LineNumberProgram::read(&mut r, 0, sections.endian, 8).unwrap();
        let mut lines = line_program.lines();
        while let Some(line) = lines.next().unwrap() {
            test::black_box(line);
        }
    });
}

#[bench]
fn line_gimli(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let debug_line = gimli::DebugLine::<gimli::LittleEndian>::new(&sections.debug_line);
        let header = gimli::LineNumberProgramHeader::new(debug_line, gimli::DebugLineOffset(0), 8).unwrap();
        let mut state_machine = gimli::StateMachine::new(&header);
        while let Some(row) = state_machine.next_row().unwrap() {
            test::black_box(row);
        }
    });
}
